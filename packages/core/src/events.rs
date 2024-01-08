use crate::{runtime::with_runtime, ScopeId};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// A wrapper around some generic data that handles the event's state
///
///
/// Prevent this event from continuing to bubble up the tree to parent elements.
///
/// # Example
///
/// ```rust, ignore
/// rsx! {
///     button {
///         onclick: move |evt: Event<MouseData>| {
///             evt.cancel_bubble();
///
///         }
///     }
/// }
/// ```
pub struct Event<T: 'static + ?Sized> {
    /// The data associated with this event
    pub data: Rc<T>,
    pub(crate) propagates: Rc<Cell<bool>>,
}

impl<T> Event<T> {
    /// Map the event data to a new type
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// rsx! {
    ///    button {
    ///       onclick: move |evt: Event<FormData>| {
    ///          let data = evt.map(|data| data.value());
    ///          assert_eq!(data.inner(), "hello world");
    ///       }
    ///    }
    /// }
    /// ```
    pub fn map<U: 'static, F: FnOnce(&T) -> U>(&self, f: F) -> Event<U> {
        Event {
            data: Rc::new(f(&self.data)),
            propagates: self.propagates.clone(),
        }
    }

    /// Prevent this event from continuing to bubble up the tree to parent elements.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             evt.cancel_bubble();
    ///         }
    ///     }
    /// }
    /// ```
    #[deprecated = "use stop_propagation instead"]
    pub fn cancel_bubble(&self) {
        self.propagates.set(false);
    }

    /// Prevent this event from continuing to bubble up the tree to parent elements.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             evt.stop_propagation();
    ///         }
    ///     }
    /// }
    /// ```
    pub fn stop_propagation(&self) {
        self.propagates.set(false);
    }

    /// Get a reference to the inner data from this event
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             let data = evt.inner.clone();
    ///             cx.spawn(async move {
    ///                 println!("{:?}", data);
    ///             });
    ///         }
    ///     }
    /// }
    /// ```
    pub fn inner(&self) -> &Rc<T> {
        &self.data
    }
}

impl<T: ?Sized> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self {
            propagates: self.propagates.clone(),
            data: self.data.clone(),
        }
    }
}

impl<T> std::ops::Deref for Event<T> {
    type Target = Rc<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiEvent")
            .field("bubble_state", &self.propagates)
            .field("data", &self.data)
            .finish()
    }
}

/// The callback type generated by the `rsx!` macro when an `on` field is specified for components.
///
/// This makes it possible to pass `move |evt| {}` style closures into components as property fields.
///
///
/// # Example
///
/// ```rust, ignore
/// rsx!{
///     MyComponent { onclick: move |evt| tracing::debug!("clicked") }
/// }
///
/// #[derive(Props)]
/// struct MyProps<'a> {
///     onclick: EventHandler<'a, MouseEvent>,
/// }
///
/// fn MyComponent(cx: Scope<'a, MyProps<'a>>) -> Element {
///     cx.render(rsx!{
///         button {
///             onclick: move |evt| cx.props.onclick.call(evt),
///         }
///     })
/// }
///
/// ```
pub struct EventHandler<'bump, T = ()> {
    pub(crate) origin: ScopeId,
    pub(super) callback: RefCell<Option<ExternalListenerCallback<'bump, T>>>,
}

impl<T> Default for EventHandler<'_, T> {
    fn default() -> Self {
        Self {
            origin: ScopeId::ROOT,
            callback: Default::default(),
        }
    }
}

type ExternalListenerCallback<'bump, T> = bumpalo::boxed::Box<'bump, dyn FnMut(T) + 'bump>;

impl<T> EventHandler<'_, T> {
    /// Call this event handler with the appropriate event type
    ///
    /// This borrows the event using a RefCell. Recursively calling a listener will cause a panic.
    pub fn call(&self, event: T) {
        if let Some(callback) = self.callback.borrow_mut().as_mut() {
            with_runtime(|rt| {
                rt.scope_stack.borrow_mut().push(self.origin);
            });
            callback(event);
            with_runtime(|rt| {
                rt.scope_stack.borrow_mut().pop();
            });
        }
    }

    /// Forcibly drop the internal handler callback, releasing memory
    ///
    /// This will force any future calls to "call" to not doing anything
    pub fn release(&self) {
        self.callback.replace(None);
    }
}
