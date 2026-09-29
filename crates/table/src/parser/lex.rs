use anyhow::{Result, bail};
use bytes::Bytes;
use std::iter::Peekable;

pub struct Lex<I>
where
    I: Iterator,
{
    input: Peekable<I>,
}

impl<I> Lex<I>
where
    I: Iterator,
{
    pub fn new(iter: I) -> Self {
        Self {
            input: iter.peekable(),
        }
    }

    pub fn peek(&mut self) -> Option<&<I as Iterator>::Item> {
        self.input.peek()
    }
}

impl<I> Lex<I>
where
    I: Iterator<Item = u8>,
{
    fn skip_whitespace(&mut self) -> Result<()> {
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_whitespace() {
                break;
            }
            self.input.next();
        }
        Ok(())
    }

    fn read_identifier(&mut self, current: u8) -> Result<Box<[u8]>> {
        if current.is_ascii_digit() {
            bail!("invalid start of an idenifier");
        }
        let mut identifier = vec![current];

        while let Some(&ch) = self.peek() {
            if !ch.is_ascii_alphabetic() && ch != b'_' && !ch.is_ascii_digit() {
                break;
            }
            let Some(n) = self.input.next() else {
                bail!("invalid eof");
            };
            identifier.push(n);
        }
        Ok(identifier.into_boxed_slice())
    }

    fn read_number(&mut self, current: u8) -> Result<Box<[u8]>> {
        if !current.is_ascii_digit() {
            bail!("not an int");
        }
        let mut number = vec![current];

        while let Some(&ch) = self.peek() {
            if !ch.is_ascii_digit() {
                break;
            }
            let Some(n) = self.input.next() else {
                bail!("invalid eof");
            };
            number.push(n);
        }
        Ok(number.into_boxed_slice())
    }
}
