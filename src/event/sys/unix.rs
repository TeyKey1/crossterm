#[cfg(all(feature = "event-stream", not(target_arch = "wasm32")))]
pub(crate) mod waker;

#[cfg(feature = "events")]
pub(crate) mod parse;
