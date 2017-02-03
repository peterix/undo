use std::fmt;
use UndoCmd;

/// Maintains a stack of `UndoCmd`s.
///
/// `UndoStack` will notice when it's state changes to either dirty or clean, and call the user
/// defined methods set in [on_clean] and [on_dirty]. This is useful if you want to trigger some
/// event when the state changes, eg. enabling and disabling buttons in an ui.
///
/// The `PopCmd` given in the examples below is defined as:
///
/// ```
/// # use undo::UndoCmd;
/// #[derive(Clone, Copy)]
/// struct PopCmd {
///     vec: *mut Vec<i32>,
///     e: Option<i32>,
/// }
///
/// impl UndoCmd for PopCmd {
///     fn redo(&mut self) {
///         self.e = unsafe {
///             let ref mut vec = *self.vec;
///             vec.pop()
///         }
///     }
///
///     fn undo(&mut self) {
///         unsafe {
///             let ref mut vec = *self.vec;
///             vec.push(self.e.unwrap());
///         }
///     }
/// }
/// ```
/// *Unsafe code is used since it is less verbose than using* `Rc` *and* `RefCell`.
///
/// [on_clean]: struct.UndoStack.html#method.on_clean
/// [on_dirty]: struct.UndoStack.html#method.on_dirty
pub struct UndoStack<'a> {
    // All commands on the stack.
    stack: Vec<Box<UndoCmd + 'a>>,
    // Current position in the stack.
    idx: usize,
    // Max amount of commands allowed on the stack.
    limit: Option<usize>,
    // Called when the state changes from dirty to clean.
    on_clean: Option<Box<FnMut() + 'a>>,
    // Called when the state changes from clean to dirty.
    on_dirty: Option<Box<FnMut() + 'a>>,
}

impl<'a> UndoStack<'a> {
    /// Creates a new `UndoStack`.
    ///
    /// # Examples
    /// ```
    /// # use undo::UndoStack;
    /// let stack = UndoStack::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        UndoStack {
            stack: Vec::new(),
            idx: 0,
            limit: None,
            on_clean: None,
            on_dirty: None,
        }
    }

    /// Creates a new `UndoStack` with a limit on how many `UndoCmd`s can be stored in the stack.
    /// If this limit is reached it will start popping of commands at the bottom of the stack when
    /// pushing new commands on to the stack. No limit is set by default which means it may grow
    /// indefinitely.
    ///
    /// The stack may remove multiple commands at a time to increase performance.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::with_limit(2);
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// stack.push(cmd);
    /// stack.push(cmd);
    /// stack.push(cmd); // Pops off the first cmd.
    ///
    /// assert!(vec.is_empty());
    ///
    /// stack.undo();
    /// stack.undo();
    /// stack.undo(); // Does nothing.
    ///
    /// assert_eq!(vec, vec![1, 2]);
    /// ```
    #[inline]
    pub fn with_limit(limit: usize) -> Self {
        UndoStack {
            stack: Vec::new(),
            idx: 0,
            limit: Some(limit),
            on_clean: None,
            on_dirty: None,
        }
    }

    /// Creates a new `UndoStack` with the specified [capacity].
    /// # Examples
    /// ```
    /// # use undo::UndoStack;
    /// let stack = UndoStack::with_capacity(10);
    /// assert_eq!(stack.capacity(), 10);
    /// ```
    ///
    /// [capacity]: https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        UndoStack {
            stack: Vec::with_capacity(capacity),
            idx: 0,
            limit: None,
            on_clean: None,
            on_dirty: None,
        }
    }

    /// Creates a new `UndoStack` with the specified capacity and limit.
    ///
    /// # Examples
    /// ```
    /// # use undo::UndoStack;
    /// let stack = UndoStack::with_capacity_and_limit(10, 10);
    /// assert_eq!(stack.capacity(), 10);
    /// assert_eq!(stack.limit(), Some(10));
    /// ```
    #[inline]
    pub fn with_capacity_and_limit(capacity: usize, limit: usize) -> Self {
        UndoStack {
            stack: Vec::with_capacity(capacity),
            idx: 0,
            limit: Some(limit),
            on_clean: None,
            on_dirty: None,
        }
    }

    /// Returns the limit of the `UndoStack`, or `None` if it has no limit.
    ///
    /// # Examples
    /// ```
    /// # use undo::UndoStack;
    /// let stack = UndoStack::with_limit(10);
    /// assert_eq!(stack.limit(), Some(10));
    ///
    /// let stack = UndoStack::new();
    /// assert_eq!(stack.limit(), None);
    /// ```
    #[inline]
    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    /// Returns the number of commands the stack can hold without reallocating.
    ///
    /// # Examples
    /// ```
    /// # use undo::UndoStack;
    /// let stack = UndoStack::with_capacity(10);
    /// assert_eq!(stack.capacity(), 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.stack.capacity()
    }

    /// Reserves capacity for at least `additional` more commands to be inserted in the given stack.
    /// The stack may reserve more space to avoid frequent reallocations.
    ///
    /// # Panics
    /// Panics if the new capacity overflows usize.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::new();
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// stack.push(cmd);
    /// stack.reserve(10);
    /// assert!(stack.capacity() >= 11);
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.stack.reserve(additional);
    }

    /// Shrinks the capacity of the `UndoStack` as much as possible.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::with_capacity(10);
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// stack.push(cmd);
    /// stack.push(cmd);
    /// stack.push(cmd);
    ///
    /// assert_eq!(stack.capacity(), 10);
    /// stack.shrink_to_fit();
    /// assert!(stack.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.stack.shrink_to_fit();
    }

    /// Sets what should happen if the state changes from dirty to clean.
    /// By default the `UndoStack` does nothing when the state changes.
    ///
    /// Note: An empty stack is clean, so the first push will not trigger this method.
    ///
    /// # Examples
    /// ```
    /// # use undo::UndoStack;
    /// let mut x = 0;
    /// let mut stack = UndoStack::new();
    /// stack.on_clean(|| x += 1);
    /// ```
    #[inline]
    pub fn on_clean<F>(&mut self, f: F)
        where F: FnMut() + 'a,
    {
        self.on_clean = Some(Box::new(f));
    }

    /// Sets what should happen if the state changes from clean to dirty.
    /// By default the `UndoStack` does nothing when the state changes.
    ///
    /// # Examples
    /// ```
    /// # use undo::UndoStack;
    /// let mut x = 0;
    /// let mut stack = UndoStack::new();
    /// stack.on_dirty(|| x += 1);
    /// ```
    #[inline]
    pub fn on_dirty<F>(&mut self, f: F)
        where F: FnMut() + 'a,
    {
        self.on_dirty = Some(Box::new(f));
    }

    /// Returns `true` if the state of the stack is clean, `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::new();
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// // An empty stack is always clean.
    /// assert!(stack.is_clean());
    ///
    /// stack.push(cmd);
    ///
    /// assert!(stack.is_clean());
    ///
    /// stack.undo();
    ///
    /// assert!(!stack.is_clean());
    /// ```
    #[inline]
    pub fn is_clean(&self) -> bool {
        self.idx == self.stack.len()
    }

    /// Returns `true` if the state of the stack is dirty, `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::new();
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// // An empty stack is always clean.
    /// assert!(!stack.is_dirty());
    ///
    /// stack.push(cmd);
    ///
    /// assert!(!stack.is_dirty());
    ///
    /// stack.undo();
    ///
    /// assert!(stack.is_dirty());
    /// ```
    #[inline]
    pub fn is_dirty(&self) -> bool {
        !self.is_clean()
    }

    /// Pushes `cmd` to the top of the stack and executes its [`redo`] method.
    /// This pops off all other commands above the active command from the stack.
    ///
    /// If `cmd`s id is equal to the top command on the stack, the two commands are merged.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::new();
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// stack.push(cmd);
    /// stack.push(cmd);
    /// stack.push(cmd);
    ///
    /// assert!(vec.is_empty());
    /// ```
    ///
    /// [`redo`]: trait.UndoCmd.html#tymethod.redo
    pub fn push<T>(&mut self, mut cmd: T)
        where T: UndoCmd + 'a,
    {
        let is_dirty = self.is_dirty();
        let len = self.idx;
        // Pop off all elements after len from stack.
        self.stack.truncate(len);
        cmd.redo();

        if len == 0 {
            self.idx += 1;
            self.stack.push(Box::new(cmd));
        } else {
            let idx = len - 1;
            match (cmd.id(), unsafe { self.stack.get_unchecked(idx).id() }) {
                (Some(id1), Some(id2)) if id1 == id2 => {
                    // Merge the command with the one on the top of the stack.
                    let cmd = MergeCmd {
                        cmd1: unsafe {
                            // Unchecked pop.
                            self.stack.set_len(idx);
                            ::std::ptr::read(self.stack.get_unchecked(idx))
                        },
                        cmd2: Box::new(cmd),
                    };
                    self.stack.push(Box::new(cmd));
                },
                _ => {
                    match self.limit {
                        Some(limit) if len == limit => {
                            // Remove ~25% of the stack at once.
                            let x = len / 4 + 1;
                            self.stack.drain(..x);
                            self.idx -= x - 1;
                        },
                        _ => self.idx += 1,
                    }
                    self.stack.push(Box::new(cmd));
                },
            }
        }

        debug_assert_eq!(self.idx, self.stack.len());
        // State is always clean after a push, check if it was dirty before.
        if is_dirty {
            if let Some(ref mut f) = self.on_clean {
                f();
            }
        }
    }

    /// Calls the [`redo`] method for the active `UndoCmd` and sets the next `UndoCmd` as the new
    /// active one.
    ///
    /// Calling this method when there are no more commands to redo does nothing.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::new();
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// stack.push(cmd);
    /// stack.push(cmd);
    /// stack.push(cmd);
    ///
    /// assert!(vec.is_empty());
    ///
    /// stack.undo();
    /// stack.undo();
    /// stack.undo();
    ///
    /// assert_eq!(vec, vec![1, 2, 3]);
    ///
    /// stack.redo();
    /// stack.redo();
    /// stack.redo();
    ///
    /// assert!(vec.is_empty());
    /// ```
    ///
    /// [`redo`]: trait.UndoCmd.html#tymethod.redo
    pub fn redo(&mut self) {
        if self.idx < self.stack.len() {
            let is_dirty = self.is_dirty();
            unsafe {
                let cmd = self.stack.get_unchecked_mut(self.idx);
                cmd.redo();
            }
            self.idx += 1;
            // Check if stack went from dirty to clean.
            if is_dirty && self.is_clean() {
                if let Some(ref mut f) = self.on_clean {
                    f();
                }
            }
        }
    }

    /// Calls the [`undo`] method for the active `UndoCmd` and sets the previous `UndoCmd` as the
    /// new active one.
    ///
    /// Calling this method when there are no more commands to undo does nothing.
    ///
    /// # Examples
    /// ```
    /// # use undo::{UndoCmd, UndoStack};
    /// # #[derive(Clone, Copy)]
    /// # struct PopCmd {
    /// #   vec: *mut Vec<i32>,
    /// #   e: Option<i32>,
    /// # }
    /// # impl UndoCmd for PopCmd {
    /// #   fn redo(&mut self) {
    /// #       self.e = unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.pop()
    /// #       }
    /// #   }
    /// #   fn undo(&mut self) {
    /// #       unsafe {
    /// #           let ref mut vec = *self.vec;
    /// #           vec.push(self.e.unwrap());
    /// #       }
    /// #   }
    /// # }
    /// let mut vec = vec![1, 2, 3];
    /// let mut stack = UndoStack::new();
    /// let cmd = PopCmd { vec: &mut vec, e: None };
    ///
    /// stack.push(cmd);
    /// stack.push(cmd);
    /// stack.push(cmd);
    ///
    /// assert!(vec.is_empty());
    ///
    /// stack.undo();
    /// stack.undo();
    /// stack.undo();
    ///
    /// assert_eq!(vec, vec![1, 2, 3]);
    /// ```
    ///
    /// [`undo`]: trait.UndoCmd.html#tymethod.undo
    pub fn undo(&mut self) {
        if self.idx > 0 {
            let is_clean = self.is_clean();
            self.idx -= 1;
            debug_assert!(self.idx < self.stack.len());
            unsafe {
                let cmd = self.stack.get_unchecked_mut(self.idx);
                cmd.undo();
            }
            // Check if stack went from clean to dirty.
            if is_clean && self.is_dirty() {
                if let Some(ref mut f) = self.on_dirty {
                    f();
                }
            }
        }
    }
}

impl<'a> Default for UndoStack<'a> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> fmt::Debug for UndoStack<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UndoStack")
            .field("stack", &self.stack)
            .field("idx", &self.idx)
            .field("limit", &self.limit)
            .finish()
    }
}

struct MergeCmd<'a> {
    cmd1: Box<UndoCmd + 'a>,
    cmd2: Box<UndoCmd + 'a>,
}

impl<'a> UndoCmd for MergeCmd<'a> {
    #[inline]
    fn redo(&mut self) {
        self.cmd1.redo();
        self.cmd2.redo();
    }

    #[inline]
    fn undo(&mut self) {
        self.cmd2.undo();
        self.cmd1.undo();
    }

    #[inline]
    fn id(&self) -> Option<u64> {
        self.cmd1.id()
    }
}

#[cfg(test)]
mod test {
    use {UndoStack, UndoCmd};

    #[derive(Clone, Copy)]
    struct PopCmd {
        vec: *mut Vec<i32>,
        e: Option<i32>,
    }

    impl UndoCmd for PopCmd {
        fn redo(&mut self) {
            self.e = unsafe {
                let ref mut vec = *self.vec;
                vec.pop()
            }
        }

        fn undo(&mut self) {
            unsafe {
                let ref mut vec = *self.vec;
                vec.push(self.e.unwrap());
            }
        }
    }

    #[test]
    fn state() {
        use std::cell::Cell;

        let x = Cell::new(0);
        let mut vec = vec![1, 2, 3];
        let mut stack = UndoStack::new();
        stack.on_clean(|| x.set(0));
        stack.on_dirty(|| x.set(1));

        let cmd = PopCmd { vec: &mut vec, e: None };
        for _ in 0..3 {
            stack.push(cmd);
        }
        assert_eq!(x.get(), 0);
        assert!(vec.is_empty());

        for _ in 0..3 {
            stack.undo();
        }
        assert_eq!(x.get(), 1);
        assert_eq!(vec, vec![1, 2, 3]);

        stack.push(cmd);
        assert_eq!(x.get(), 0);
        assert_eq!(vec, vec![1, 2]);

        stack.undo();
        assert_eq!(x.get(), 1);
        assert_eq!(vec, vec![1, 2, 3]);

        stack.redo();
        assert_eq!(x.get(), 0);
        assert_eq!(vec, vec![1, 2]);
    }

    #[test]
    fn limit() {
        let mut vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut stack = UndoStack::with_limit(9);

        let cmd = PopCmd { vec: &mut vec, e: None };

        for _ in 0..10 {
            stack.push(cmd);
        }

        assert!(vec.is_empty());
        assert_eq!(stack.stack.len(), 7);
    }
}
