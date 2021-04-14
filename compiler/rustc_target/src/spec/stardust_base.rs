use crate::spec::TargetOptions;
use crate::spec::{LinkerFlavor, PanicStrategy};

pub fn opts() -> TargetOptions {
    TargetOptions {
        is_builtin: false,
        os: "stardust".to_string(),
        linker_flavor: LinkerFlavor::Gcc,
        executables: true,
        linker: Some("stardust-rs-link".to_string()),
        no_default_libraries: true,
        allow_asm: true,
        dynamic_linking: false,
        panic_strategy: PanicStrategy::Abort,
        os_family: None,
        ..Default::default()
    }
}
