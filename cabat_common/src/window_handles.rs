//====================================================================

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use send_sync::WasmNotSendSync;

//====================================================================

// Code mostly yoinked from wgpu

pub trait WindowHandle: HasWindowHandle + HasDisplayHandle + WasmNotSendSync {}

impl<T> WindowHandle for T where T: HasWindowHandle + HasDisplayHandle + WasmNotSendSync {}

mod send_sync {
    pub trait WasmNotSendSync: WasmNotSend + WasmNotSync {}

    impl<T: WasmNotSend + WasmNotSync> WasmNotSendSync for T {}

    #[cfg(any(not(target_arch = "wasm32"),))]
    pub trait WasmNotSend: Send {}

    #[cfg(any(not(target_arch = "wasm32"),))]
    impl<T: Send> WasmNotSend for T {}

    #[cfg(not(any(not(target_arch = "wasm32"),)))]
    pub trait WasmNotSend {}

    #[cfg(not(any(not(target_arch = "wasm32"),)))]
    impl<T> WasmNotSend for T {}

    #[cfg(any(not(target_arch = "wasm32"),))]
    pub trait WasmNotSync: Sync {}

    #[cfg(any(not(target_arch = "wasm32"),))]

    impl<T: Sync> WasmNotSync for T {}
    #[cfg(not(any(not(target_arch = "wasm32"),)))]
    pub trait WasmNotSync {}

    #[cfg(not(any(not(target_arch = "wasm32"),)))]
    impl<T> WasmNotSync for T {}
}

//====================================================================
