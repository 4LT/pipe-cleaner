use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use wasmtime::{Engine, Linker, Module, Store, Val};

pub struct Host {
    engine: Engine,
    module: Module,
    linker: Linker<()>,
}

impl Host {
    pub fn new<T: Into<PathBuf>>(path: T) -> Result<Self, String> {
        let engine = Default::default();

        let mut file = File::open(path.into()).map_err(|e| e.to_string())?;

        let mut bytes = Vec::new();

        file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;

        let module = Module::new(&engine, bytes).map_err(|e| e.to_string())?;

        let linker = Linker::new(&engine);

        Ok(Host {
            engine,
            module,
            linker,
        })
    }

    pub fn run(&self) -> Result<(), String> {
        let mut store = Store::new(&self.engine, ());

        let instance = self
            .linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| e.to_string())?;

        let add = instance
            .get_typed_func::<(u32, u32), u32>(&mut store, "add")
            .map_err(|e| e.to_string())?;

        let panic_report_address = instance
            .get_global(&mut store, "PIPECLEANER_panic_report")
            .ok_or(String::from("Export not found"))?;

        let panic_report_address = match panic_report_address.get(&mut store) {
            Val::I32(i) => Some(i as usize),
            _ => None,
        };

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
