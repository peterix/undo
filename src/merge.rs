use crate::{Command, Merge};
#[cfg(feature = "display")]
use std::fmt;
use std::{
    iter::{FromIterator, IntoIterator},
    vec::IntoIter,
};

/// Macro for merging commands.
///
/// # Examples
/// ```
/// # use undo::*;
/// # struct Add(char);
/// # impl Command<String> for Add {
/// #     fn apply(&mut self, s: &mut String) -> undo::Result {
/// #         s.push(self.0);
/// #         Ok(())
/// #     }
/// #     fn undo(&mut self, s: &mut String) -> undo::Result {
/// #         self.0 = s.pop().ok_or("`s` is empty")?;
/// #         Ok(())
/// #     }
/// # }
/// # fn main() -> undo::Result {
/// let mut record = Record::default();
/// record.apply(merge![Add('a'), Add('b'), Add('c')])?;
/// assert_eq!(record.as_receiver(), "abc");
/// record.undo().unwrap()?;
/// assert_eq!(record.as_receiver(), "");
/// record.redo().unwrap()?;
/// assert_eq!(record.as_receiver(), "abc");
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! merge {
    ($($commands:expr),*) => {{
        let mut merged = $crate::Merged::new();
        $(merged.push($commands);)*
        merged
    }};
}

/// Merged commands.
///
/// Commands that have been merged are all executed in the order they was merged in when applied.
///
/// The [`merge!`](macro.merge.html) macro can be used for convenience when merging commands.
pub struct Merged<R> {
    commands: Vec<Box<dyn Command<R>>>,
    #[cfg(feature = "display")]
    text: Option<String>,
}

impl<R> Merged<R> {
    /// Returns an empty command.
    #[inline]
    pub fn new() -> Merged<R> {
        Merged {
            commands: vec![],
            #[cfg(feature = "display")]
            text: None,
        }
    }

    /// Merges `cmd1` and `cmd2` into a single command.
    #[inline]
    pub fn merge(cmd1: impl Command<R> + 'static, cmd2: impl Command<R> + 'static) -> Merged<R> {
        Merged {
            commands: vec![Box::new(cmd1), Box::new(cmd2)],
            #[cfg(feature = "display")]
            text: None,
        }
    }

    /// Creates a new command with the provided text.
    #[inline]
    #[cfg(feature = "display")]
    pub fn with_text(text: impl Into<String>) -> Merged<R> {
        Merged {
            commands: vec![],
            #[cfg(feature = "display")]
            text: Some(text.into()),
        }
    }

    /// Merges `self` with `command`.
    #[inline]
    pub fn push(&mut self, command: impl Command<R> + 'static) {
        self.commands.push(Box::new(command));
    }

    /// Merges `self` with `command` and returns the merged command.
    #[inline]
    pub fn join(mut self, command: impl Command<R> + 'static) -> Merged<R> {
        self.push(command);
        self
    }

    /// Returns the amount of commands that have been merged.
    #[inline]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns `true` if no commands have been merged.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Returns the text for the merged commands.
    #[inline]
    #[cfg(feature = "display")]
    pub fn text(&self) -> Option<&str> {
        self.text.as_ref().map(String::as_str)
    }

    /// Sets the text for the merged commands.
    #[inline]
    #[cfg(feature = "display")]
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = Some(text.into());
    }
}

impl<R> Command<R> for Merged<R> {
    #[inline]
    fn apply(&mut self, receiver: &mut R) -> crate::Result {
        for command in &mut self.commands {
            command.apply(receiver)?;
        }
        Ok(())
    }

    #[inline]
    fn undo(&mut self, receiver: &mut R) -> crate::Result {
        for command in self.commands.iter_mut().rev() {
            command.undo(receiver)?;
        }
        Ok(())
    }

    #[inline]
    fn redo(&mut self, receiver: &mut R) -> crate::Result {
        for command in &mut self.commands {
            command.redo(receiver)?;
        }
        Ok(())
    }

    #[inline]
    fn merge(&self) -> Merge {
        self.commands.first().map_or(Merge::Yes, Command::merge)
    }

    #[inline]
    fn is_dead(&self) -> bool {
        self.commands.iter().any(Command::is_dead)
    }
}

impl<R> Default for Merged<R> {
    #[inline]
    fn default() -> Self {
        Merged::new()
    }
}

impl<R, C: Command<R> + 'static> FromIterator<C> for Merged<R> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = C>>(commands: T) -> Self {
        Merged {
            commands: commands.into_iter().map(|c| Box::new(c) as _).collect(),
            #[cfg(feature = "display")]
            text: None,
        }
    }
}

impl<R> IntoIterator for Merged<R> {
    type Item = Box<dyn Command<R>>;
    type IntoIter = IntoIter<Self::Item>;

    #[inline]
    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.commands.into_iter()
    }
}

impl<R, C: Command<R> + 'static> Extend<C> for Merged<R> {
    #[inline]
    fn extend<T: IntoIterator<Item = C>>(&mut self, iter: T) {
        self.commands
            .extend(iter.into_iter().map(|c| Box::new(c) as _));
    }
}

#[cfg(feature = "display")]
impl<R> fmt::Debug for Merged<R> {
    #[inline]
    #[cfg(not(feature = "display"))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Merged")
            .field("commands", &self.commands)
            .finish()
    }

    #[inline]
    #[cfg(feature = "display")]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Merged")
            .field("commands", &self.commands)
            .field("text", &self.text)
            .finish()
    }
}

#[cfg(feature = "display")]
impl<R> fmt::Display for Merged<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.text {
            Some(text) => f.write_str(text),
            None => {
                if let Some((first, commands)) = self.commands.split_first() {
                    (first as &dyn fmt::Display).fmt(f)?;
                    for command in commands {
                        write!(f, "\n\n{}", command)?;
                    }
                }
                Ok(())
            }
        }
    }
}
