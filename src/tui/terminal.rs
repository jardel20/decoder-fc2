//! ANSI terminal lifecycle. Requires a VT-compatible Unix terminal or modern
//! Windows Terminal; legacy Windows consoles without VT support are unsupported.
//! Resize is observed on the next key press. `console` restores the input mode
//! after each raw read; our guard restores the display on errors and unwinding.

use std::io::{self, IsTerminal, Write};

use console::{Key, Term};

use crate::error::{Error, Result};

const ENTER: &[u8] = b"\x1b[?1049h\x1b[?25l";
const LEAVE: &[u8] = b"\x1b[0m\x1b[?1049l\x1b[?25h";

struct Screen<W: Write> {
    writer: W,
    active: bool,
}

impl<W: Write> Screen<W> {
    fn enter(writer: W) -> io::Result<Self> {
        // Arm before the first write so even a partial enter sequence is undone.
        let mut screen = Self {
            writer,
            active: true,
        };
        screen.writer.write_all(ENTER)?;
        screen.writer.flush()?;
        Ok(screen)
    }

    fn frame(&mut self, text: &str) -> io::Result<()> {
        self.writer.write_all(b"\x1b[H\x1b[2J")?;
        self.writer.write_all(text.as_bytes())?;
        self.writer.flush()
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.active {
            self.writer.write_all(LEAVE)?;
            self.writer.flush()?;
            self.active = false;
        }
        Ok(())
    }
}

impl<W: Write> Drop for Screen<W> {
    fn drop(&mut self) {
        // Best effort during unwinding; avoid panicking again on an I/O error.
        let _ = self.finish();
    }
}

/// Draws the initial frame, then dispatches raw keys (including Ctrl-C).
/// `None` from the callback ends the session. Dimensions are width, height.
/// Pipes are rejected before reading, preventing an unattended input loop.
pub fn run_loop<F>(mut step: F) -> Result<()>
where
    F: FnMut(Option<Key>, u16, u16) -> Result<Option<String>>,
{
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::NotConnected,
            "A TUI exige entrada e saída ligadas a um terminal; use os comandos CLI em pipes.",
        )));
    }
    if std::env::var_os("TERM").is_some_and(|value| value == "dumb") {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "A TUI exige um terminal com suporte a sequências ANSI/VT.",
        )));
    }

    let term = Term::stdout();
    // On Windows this feature probe asks console to enable VT processing when
    // supported. NO_COLOR can suppress that probe; Windows Terminal already
    // supports VT. We never change process environment or send OS signals.
    let _ = term.features().colors_supported();
    let mut screen = Screen::enter(term.clone())?;
    let mut key = None;
    loop {
        let (height, width) = term.size();
        match step(key, width, height)? {
            Some(frame) => screen.frame(&frame)?,
            None => return Ok(screen.finish()?),
        }
        key = Some(term.read_key_raw()?);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct Capture(Rc<RefCell<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn finish_restores_once_and_frames_clear_stale_content() {
        let bytes = Rc::new(RefCell::new(Vec::new()));
        let mut screen = Screen::enter(Capture(bytes.clone())).expect("enter");
        screen.frame("frame").expect("frame");
        screen.finish().expect("leave");
        drop(screen);
        let expected = [ENTER, b"\x1b[H\x1b[2Jframe", LEAVE].concat();
        assert_eq!(*bytes.borrow(), expected);
    }

    #[test]
    fn panic_unwinding_restores_display() {
        let bytes = Rc::new(RefCell::new(Vec::new()));
        let writer = Capture(bytes.clone());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _screen = Screen::enter(writer).expect("enter");
            panic!("simulated application panic");
        }));
        assert!(result.is_err());
        assert_eq!(*bytes.borrow(), [ENTER, LEAVE].concat());
    }

    #[test]
    fn callback_error_scope_restores_display() {
        let bytes = Rc::new(RefCell::new(Vec::new()));
        let fail = || -> io::Result<()> {
            let _screen = Screen::enter(Capture(bytes.clone()))?;
            Err(io::Error::other("simulated callback failure"))
        };
        assert!(fail().is_err());
        assert_eq!(*bytes.borrow(), [ENTER, LEAVE].concat());
    }

    #[test]
    fn failed_entry_also_attempts_restoration() {
        struct FailOnce {
            capture: Capture,
            fail: bool,
        }

        impl Write for FailOnce {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.fail {
                    self.fail = false;
                    return Err(io::Error::other("simulated display failure"));
                }
                self.capture.write(bytes)
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let bytes = Rc::new(RefCell::new(Vec::new()));
        let result = Screen::enter(FailOnce {
            capture: Capture(bytes.clone()),
            fail: true,
        });
        assert!(result.is_err());
        assert_eq!(*bytes.borrow(), LEAVE);
    }
}
