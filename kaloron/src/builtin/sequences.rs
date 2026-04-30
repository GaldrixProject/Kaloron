// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::*;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet, LinkedList, VecDeque};
use std::hash::Hash;
use std::iter::Peekable;

#[inline(never)]
fn sequence_exhausted_err() -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "sequence value was exhausted before send completed"
    ))
}

#[inline(never)]
fn duplicate_set_element_received_err() -> anyhow::Result<()> {
    Err(anyhow::anyhow!("duplicate set element received"))
}

// Consolidates the repeated `SequenceSend` implementations for sequence-like
// containers (Vec, Box<[T]>, [T; N], VecDeque, LinkedList, HashSet, BTreeSet).
struct SeqSend<'a, T: 'a, I>
where
    I: Iterator<Item = &'a T>,
{
    len: usize,
    iter: RefCell<Peekable<I>>,
}

impl<'a, T, I> SeqSend<'a, T, I>
where
    I: Iterator<Item = &'a T>,
{
    fn new(len: usize, iter: I) -> Self {
        Self {
            len,
            iter: RefCell::new(iter.peekable()),
        }
    }
}

impl<'a, T, I> SequenceSend for SeqSend<'a, T, I>
where
    I: Iterator<Item = &'a T>,
    T: TypeShape,
{
    fn len(&self) -> Option<usize> {
        Some(self.len)
    }

    fn has_next(&self) -> bool {
        self.iter.borrow_mut().peek().is_some()
    }

    fn visit_next(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()> {
        match self.iter.borrow_mut().next() {
            Some(value) => visitor.visit(value),
            None => sequence_exhausted_err(),
        }
    }
}

#[inline(never)]
fn error_len_missing() -> anyhow::Result<usize> {
    anyhow::bail!("sequence length was not found when required")
}

#[inline(always)]
fn expect_len(o: Option<usize>) -> anyhow::Result<usize> {
    match o {
        None => error_len_missing(),
        Some(l) => Ok(l),
    }
}

impl<T: TypeShape> TypeShape for Vec<T> {
    const SCHEMA: &'static Schema<'static> = &Schema::Seq(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_sequence(&SeqSend::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<T> {
            values: Vec<T>,
        }

        impl<T: TypeShape> SequenceRecv for Recv<T> {
            fn len(&mut self, size: Option<usize>) -> anyhow::Result<()> {
                self.values.reserve(expect_len(size)?);
                Ok(())
            }

            fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                self.values.push(visitor.visit::<T>()?);
                Ok(())
            }
        }

        let mut helper = Recv { values: Vec::new() };
        recv.accept_sequence(&mut helper)?;
        Ok(helper.values)
    }
}

impl<T: TypeShape> TypeShape for Box<[T]> {
    const SCHEMA: &'static Schema<'static> = &Schema::Seq(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_sequence(&SeqSend::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<T> {
            values: Vec<T>,
        }

        impl<T: TypeShape> SequenceRecv for Recv<T> {
            fn len(&mut self, size: Option<usize>) -> anyhow::Result<()> {
                self.values.reserve(expect_len(size)?);
                Ok(())
            }

            fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                self.values.push(visitor.visit::<T>()?);
                Ok(())
            }
        }

        let mut helper = Recv { values: Vec::new() };
        recv.accept_sequence(&mut helper)?;
        Ok(helper.values.into_boxed_slice())
    }
}

impl<T: TypeShape, const N: usize> TypeShape for [T; N] {
    const SCHEMA: &'static Schema<'static> = &Schema::Seq(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_sequence(&SeqSend::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<T, const N: usize> {
            values: Vec<T>,
        }

        impl<T: TypeShape, const N: usize> SequenceRecv for Recv<T, N> {
            fn len(&mut self, size: Option<usize>) -> anyhow::Result<()> {
                let size = expect_len(size)?;
                if size != N {
                    return Err(anyhow::anyhow!(
                        "array length mismatch: expected {}, got {}",
                        N,
                        size
                    ));
                }
                self.values.reserve(size);
                Ok(())
            }

            fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                self.values.push(visitor.visit::<T>()?);
                Ok(())
            }
        }

        let mut helper = Recv::<T, N> {
            values: Vec::with_capacity(N),
        };
        recv.accept_sequence(&mut helper)?;

        if helper.values.len() != N {
            return Err(anyhow::anyhow!(
                "array length mismatch: expected {}, got {}",
                N,
                helper.values.len()
            ));
        }

        helper.values.try_into().map_err(|values: Vec<T>| {
            anyhow::anyhow!(
                "array length mismatch: expected {}, got {}",
                N,
                values.len()
            )
        })
    }
}

impl<T: TypeShape> TypeShape for VecDeque<T> {
    const SCHEMA: &'static Schema<'static> = &Schema::Seq(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_sequence(&SeqSend::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<T> {
            values: VecDeque<T>,
        }

        impl<T: TypeShape> SequenceRecv for Recv<T> {
            fn len(&mut self, size: Option<usize>) -> anyhow::Result<()> {
                self.values.reserve(expect_len(size)?);
                Ok(())
            }

            fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                self.values.push_back(visitor.visit::<T>()?);
                Ok(())
            }
        }

        let mut helper = Recv {
            values: VecDeque::new(),
        };
        recv.accept_sequence(&mut helper)?;
        Ok(helper.values)
    }
}

impl<T: TypeShape> TypeShape for LinkedList<T> {
    const SCHEMA: &'static Schema<'static> = &Schema::Seq(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_sequence(&SeqSend::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<T> {
            values: LinkedList<T>,
        }

        impl<T: TypeShape> SequenceRecv for Recv<T> {
            fn len(&mut self, size: Option<usize>) -> anyhow::Result<()> {
                let _ = expect_len(size)?;
                Ok(())
            }

            fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                self.values.push_back(visitor.visit::<T>()?);
                Ok(())
            }
        }

        let mut helper = Recv {
            values: LinkedList::new(),
        };
        recv.accept_sequence(&mut helper)?;
        Ok(helper.values)
    }
}

// --- Set types merged in to reuse the local SeqSend and avoid visibility
//     issues with sibling modules.

impl<T> TypeShape for HashSet<T>
where
    T: TypeShape + Eq + Hash,
{
    const SCHEMA: &'static Schema<'static> = &Schema::Seq(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_sequence(&SeqSend::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<T> {
            values: HashSet<T>,
        }

        impl<T> SequenceRecv for Recv<T>
        where
            T: TypeShape + Eq + Hash,
        {
            fn len(&mut self, size: Option<usize>) -> anyhow::Result<()> {
                if let Some(size) = size {
                    self.values.reserve(size);
                }
                Ok(())
            }

            fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                match self.values.insert(visitor.visit::<T>()?) {
                    true => Ok(()),
                    false => duplicate_set_element_received_err(),
                }
            }
        }

        let mut helper = Recv {
            values: HashSet::new(),
        };
        recv.accept_sequence(&mut helper)?;
        Ok(helper.values)
    }
}

impl<T> TypeShape for BTreeSet<T>
where
    T: TypeShape + Ord,
{
    const SCHEMA: &'static Schema<'static> = &Schema::Seq(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_sequence(&SeqSend::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<T> {
            values: BTreeSet<T>,
        }

        impl<T> SequenceRecv for Recv<T>
        where
            T: TypeShape + Ord,
        {
            fn len(&mut self, _size: Option<usize>) -> anyhow::Result<()> {
                Ok(())
            }

            fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                match self.values.insert(visitor.visit::<T>()?) {
                    true => Ok(()),
                    false => duplicate_set_element_received_err(),
                }
            }
        }

        let mut helper = Recv {
            values: BTreeSet::new(),
        };
        recv.accept_sequence(&mut helper)?;
        Ok(helper.values)
    }
}
