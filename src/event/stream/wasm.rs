use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{Arc, RwLock as StdRwLock},
    task::{Context, Poll, Waker},
};

use futures_core::Stream;
use wasm_bindgen::{prelude::Closure, JsCast};
use xterm_js_rs::xterm::{Disposable, Terminal};

use super::super::{
    sys::unix::parse::parse_event, // TODO: spin out parse instead...
    Event,
    InternalEvent,
};

type RwLock<T> = StdRwLock<T>;

pub struct EventStream<'t> {
    // On `Drop` this will automatically get unregistered.
    data_event_listener_handle: Disposable,
    resize_event_listener_handle: Disposable,
    // We store these here because these callbacks need to be somewhere in
    // memory while we're still listening for events and since we stop listening
    // by dropping this type, things work out nicely.
    data_event_closure: Closure<dyn FnMut(String)>,
    resize_event_closure: Closure<dyn FnMut(Terminal)>,

    waker: Arc<RwLock<Option<Waker>>>,
    events: Arc<RwLock<VecDeque<std::io::Result<InternalEvent>>>>,

    terminal: &'t Terminal,
}

// Note: another way to do this would be to just hold a reference to the
// Terminal and to register the callbacks (with the waker built-in) in
// `poll_next`.
//
// This would let us get rid of the `Arc<RwLock<Option<Waker>>>` thing and
// it'd make us hold onto the `Terminal` to make sure that it doesn't go out
// of scope before the `EventStream` does (we do this anyways but we don't
// actually do anything with the terminal).

impl<'t> EventStream<'t> {
    // Warning: This does not check if the terminal already has an event stream
    // registered to it. If it does, registering a new EventStream to the
    // terminal will break the existing EventStream.
    pub fn new(term: &'t Terminal) -> Self {
        let waker = Arc::new(RwLock::new(None));
        let events = Arc::new(RwLock::new(VecDeque::with_capacity(64)));

        let (data_event_closure, data_event_listener_handle) = {
            let (waker, events) = (waker.clone(), events.clone());

            let clos: Box<dyn FnMut(_)> = Box::new(move |data: String| {
                let mut events = events.write().unwrap();
                let mut buffer = Vec::with_capacity(10);
                let bytes = data.as_bytes();

                for (idx, byte) in bytes.iter().enumerate() {
                    let more = idx + 1 < bytes.len();
                    buffer.push(*byte);

                    match parse_event(&buffer, more) {
                        Ok(Some(ev)) => {
                            events.push_back(Ok(ev));
                            buffer.clear();
                        }
                        Ok(None) => {
                            // Add some more bytes and try again..
                        }
                        Err(err) => {
                            // Store the error and clear the buffer.
                            events.push_back(Err(err));
                            buffer.clear();
                        }
                    }
                }

                waker
                    .read()
                    .unwrap()
                    .as_ref()
                    .map(|w: &Waker| w.wake_by_ref());
            });
            let clos = Closure::wrap(clos);
            let handle = term.on_data(clos.as_ref().unchecked_ref()).into();

            (clos, handle)
        };

        let (resize_event_closure, resize_event_listener_handle) = {
            let (waker, events) = (waker.clone(), events.clone());
            let clos: Box<dyn FnMut(_)> = Box::new(move |ev: Terminal| {
                let mut events = events.write().unwrap();
                events.push_back(Ok(InternalEvent::Event(Event::Resize(
                    ev.get_cols().try_into().unwrap_or_else(|_| 0),
                    ev.get_rows().try_into().unwrap_or_else(|_| 0),
                ))));

                waker.read().unwrap().as_ref().map(|w| w.wake_by_ref());
            });
            let clos = Closure::wrap(clos);
            let handle = term.on_resize(clos.as_ref().unchecked_ref()).into();

            (clos, handle)
        };

        Self {
            data_event_listener_handle,
            resize_event_listener_handle,

            data_event_closure,
            resize_event_closure,

            waker,
            events,

            terminal: term,
        }
    }
}

impl<'t> Stream for EventStream<'t> {
    type Item = std::io::Result<Event>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.waker.read().unwrap().is_none() {
            *self.waker.write().unwrap() = Some(cx.waker().clone())
        }

        let mut events = self.events.write().unwrap();

        // Filter out CursorPosition events here:
        loop {
            match events.pop_front() {
                Some(Err(err)) => break Poll::Ready(Some(Err(err))),
                Some(Ok(InternalEvent::Event(ev))) => break Poll::Ready(Some(Ok(ev))),
                Some(Ok(InternalEvent::CursorPosition(_, _))) => continue,
                Some(Ok(_)) => continue, // TODO
                None => break Poll::Pending,
            }
        }
    }
}
