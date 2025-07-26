use crate::wasm_entity::{Entity, Handle};
use crate::world::WasmWorld;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

use wasmtime::{Caller, Engine, Extern, Linker, Module, Store, Val};

pub struct Host {
    engine: Engine,
    module: Module,
    linker: Linker<Rc<RefCell<WasmWorld>>>,
}

impl Host {
    pub fn new<T: Into<PathBuf>>(path: T) -> Result<Self, String> {
        let engine = Default::default();

        let mut file = File::open(path.into()).map_err(|e| e.to_string())?;

        let mut bytes = Vec::new();

        file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;

        let module = Module::new(&engine, bytes).map_err(|e| e.to_string())?;

        let mut linker = Linker::new(&engine);

        linker
            .func_wrap("env", "PIPECLEANER_create_entity", create_entity)
            .map_err(|e| e.to_string())?;

        linker
            .func_wrap("env", "PIPECLEANER_get_entity", get_entity)
            .map_err(|e| e.to_string())?;

        linker
            .func_wrap(
                "env",
                "PIPECLEANER_write_entity_back",
                write_entity_back,
            )
            .map_err(|e| e.to_string())?;

        linker
            .func_wrap("env", "PIPECLEANER_remove_entity", remove_entity)
            .map_err(|e| e.to_string())?;

        Ok(Host {
            engine,
            module,
            linker,
        })
    }

    pub fn run(&self) -> Result<(), String> {
        let wasm_world = Rc::new(RefCell::new(WasmWorld::default()));
        let mut store = Store::new(&self.engine, Rc::clone(&wasm_world));

        let instance = self
            .linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| e.to_string())?;

        let add = instance
            .get_typed_func::<(u32, u32), u32>(&mut store, "add")
            .map_err(|e| e.to_string())?;

        let create_and_write = instance
            .get_typed_func::<(), u64>(&mut store, "create_and_write_entity")
            .map_err(|e| e.to_string())?;

        let read_and_remove = instance
            .get_typed_func::<u64, f32>(&mut store, "read_and_remove_entity")
            .map_err(|e| e.to_string())?;

        let panic_report_address = instance
            .get_global(&mut store, "PIPECLEANER_panic_report")
            .ok_or(String::from("Export not found"))?;

        let panic_report_address = match panic_report_address.get(&mut store) {
            Val::I32(i) => Some(i as usize),
            _ => None,
        };

        let handle = create_and_write
            .call(&mut store, ())
            .map_err(|e| e.to_string())?;

        let handle = Handle::from_bits(handle).unwrap();
        println!(
            "Handle created, id: {}, index: {}",
            handle.id(),
            handle.index()
        );
        let handle = handle.bits();

        let sum = read_and_remove
            .call(&mut store, handle)
            .map_err(|e| e.to_string())?;

        println!("Entity position sum: {}", sum);

        let arg_lists = [(3, 4), (255, 1)];

        for (x, y) in arg_lists {
            let result = add.call(&mut store, (x, y));

            match result {
                Ok(sum) => println!("Calculated sum: {}", sum),
                _ => (),
            }
        }

        if let Some(address) = panic_report_address {
            eprintln!("Panic report address: {address}");

            let memory = instance.get_memory(&mut store, "memory").unwrap();
            let mut bytes = [0u8; 4 + 4 + 256];
            memory
                .read(&mut store, address, &mut bytes)
                .map_err(|e| e.to_string())?;

            let code = u32::from_le_bytes(bytes[0..4].as_chunks().0[0]);
            let msg_len =
                u32::from_le_bytes(bytes[4..8].as_chunks().0[0]) as usize;

            eprintln!("Error code: {code}");
            eprintln!("Message length: {msg_len}");

            if code == 1 || code == 2 {
                let message_slice = &bytes[8..][..msg_len];

                eprintln!(
                    "{}",
                    std::str::from_utf8(message_slice)
                        .unwrap_or("Bad UTF-8 string")
                );
            } else {
                eprintln!("No panic detected");
            }
        }

        Ok(())
    }
}

fn create_entity(caller: Caller<'_, Rc<RefCell<WasmWorld>>>) -> u64 {
    caller.data().borrow_mut().create_entity().bits()
}

fn get_entity(
    mut caller: Caller<'_, Rc<RefCell<WasmWorld>>>,
    handle_bits: u64,
    address: u32,
) -> u32 {
    let address = address as usize;

    if let Some(handle) = Handle::from_bits(handle_bits)
        && {
            let world = Rc::clone(caller.data());

            let memory = match caller.get_export("memory").unwrap() {
                Extern::Memory(m) => m.data_mut(&mut caller),
                _ => panic!("Expected export to be memory"),
            };

            world.borrow().write_entity_to_guest(
                handle,
                &mut memory[address..address + size_of::<Entity>()],
            )
        }
    {
        0
    } else {
        1
    }
}

fn write_entity_back(
    mut caller: Caller<'_, Rc<RefCell<WasmWorld>>>,
    handle_bits: u64,
    address: u32,
) -> u32 {
    let address = address as usize;

    if let Some(handle) = Handle::from_bits(handle_bits)
        && {
            let world = Rc::clone(caller.data());

            let memory = match caller.get_export("memory").unwrap() {
                Extern::Memory(m) => m.data(&caller),
                _ => panic!("Expected export to be memory"),
            };

            world.borrow_mut().read_entity_from_guest(
                handle,
                &memory[address..address + size_of::<Entity>()],
            )
        }
    {
        0
    } else {
        1
    }
}

fn remove_entity(
    caller: Caller<'_, Rc<RefCell<WasmWorld>>>,
    handle_bits: u64,
) -> u32 {
    if let Some(handle) = Handle::from_bits(handle_bits)
        && caller.data().borrow_mut().remove_entity(handle)
    {
        0
    } else {
        1
    }
}
