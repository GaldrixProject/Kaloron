// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::*;
use std::iter::Peekable;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

#[inline(never)]
fn map_exhausted_err() -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "map value was exhausted before send completed"
    ))
}

#[inline(never)]
fn duplicate_map_key_received_err() -> anyhow::Result<()> {
    Err(anyhow::anyhow!("duplicate map key received"))
}

// Private generic map send adapter that works with any iterator yielding
// (&'a K, &'a V). Uses Peekable so `has_next` can be implemented.
struct MapSendIter<'a, K: 'a, V: 'a, I>
where
    I: Iterator<Item = (&'a K, &'a V)>,
{
    len: usize,
    iter: RefCell<Peekable<I>>,
}

impl<'a, K, V, I> MapSendIter<'a, K, V, I>
where
    I: Iterator<Item = (&'a K, &'a V)>,
{
    fn new(len: usize, iter: I) -> Self {
        Self {
            len,
            iter: RefCell::new(iter.peekable()),
        }
    }
}

impl<'a, K, V, I> MapSend for MapSendIter<'a, K, V, I>
where
    I: Iterator<Item = (&'a K, &'a V)>,
    K: TypeShape,
    V: TypeShape,
{
    fn len(&self) -> Option<usize> {
        Some(self.len)
    }

    fn has_next(&self) -> bool {
        self.iter.borrow_mut().peek().is_some()
    }

    fn visit_next(
        &self,
        visitor_k: &mut impl SendVisitor,
        visitor_v: &mut impl SendVisitor,
    ) -> anyhow::Result<()> {
        match self.iter.borrow_mut().next() {
            Some((k, v)) => {
                visitor_k.visit(k)?;
                visitor_v.visit(v)
            }
            None => map_exhausted_err(),
        }
    }
}

impl<K, V> TypeShape for HashMap<K, V>
where
    K: TypeShape + Eq + Hash,
    V: TypeShape,
{
    const SCHEMA: &'static Schema<'static> = &Schema::Map(K::SCHEMA, V::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_map(&MapSendIter::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<K, V> {
            values: HashMap<K, V>,
        }

        impl<K, V> MapRecv for Recv<K, V>
        where
            K: TypeShape + Eq + Hash,
            V: TypeShape,
        {
            fn len(&mut self, size: usize) -> anyhow::Result<()> {
                self.values.reserve(size);
                Ok(())
            }

            fn visit_next(
                &mut self,
                k: &mut impl RecvVisitor,
                v: &mut impl RecvVisitor,
            ) -> anyhow::Result<()> {
                match self.values.insert(k.visit::<K>()?, v.visit::<V>()?) {
                    None => Ok(()),
                    Some(_) => duplicate_map_key_received_err(),
                }
            }
        }

        let mut helper = Recv {
            values: HashMap::new(),
        };
        recv.accept_map(&mut helper)?;
        Ok(helper.values)
    }
}

impl<K, V> TypeShape for BTreeMap<K, V>
where
    K: TypeShape + Ord,
    V: TypeShape,
{
    const SCHEMA: &'static Schema<'static> = &Schema::Map(K::SCHEMA, V::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_map(&MapSendIter::new(self.len(), self.iter()))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        struct Recv<K, V> {
            values: BTreeMap<K, V>,
        }

        impl<K, V> MapRecv for Recv<K, V>
        where
            K: TypeShape + Ord,
            V: TypeShape,
        {
            fn len(&mut self, _size: usize) -> anyhow::Result<()> {
                Ok(())
            }

            fn visit_next(
                &mut self,
                k: &mut impl RecvVisitor,
                v: &mut impl RecvVisitor,
            ) -> anyhow::Result<()> {
                match self.values.insert(k.visit::<K>()?, v.visit::<V>()?) {
                    None => Ok(()),
                    Some(_) => duplicate_map_key_received_err(),
                }
            }
        }

        let mut helper = Recv {
            values: BTreeMap::new(),
        };
        recv.accept_map(&mut helper)?;
        Ok(helper.values)
    }
}
