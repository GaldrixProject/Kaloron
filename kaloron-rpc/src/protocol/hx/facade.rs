// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use anyhow::{anyhow, bail};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::{Arc, RwLock};
use typeid::ConstTypeId;

struct AnyArcVt {
    clone: fn(raw: usize) -> usize,
    drop: fn(raw: usize),
}

struct AnyArcVtSt<T>(PhantomData<T>);

impl<T> AnyArcVtSt<T> {
    pub const VT: AnyArcVt = AnyArcVt {
        clone: |raw| unsafe {
            let p = Arc::from_raw(raw as *const T);
            let c = p.clone();
            let _ = ManuallyDrop::new(p);
            Arc::into_raw(c) as usize
        },
        drop: |raw| unsafe {
            let _ = Arc::from_raw(raw as *const T);
        },
    };
}

struct AnyArc {
    raw: usize,
    vtb: &'static AnyArcVt,
}

impl AnyArc {
    fn new<T>(p: Arc<T>) -> Self {
        Self {
            raw: Arc::into_raw(p) as usize,
            vtb: &AnyArcVtSt::<T>::VT,
        }
    }

    fn recover<T>(self) -> Arc<T> {
        unsafe { Arc::from_raw(self.raw as *const T) }
    }
}

impl Clone for AnyArc {
    fn clone(&self) -> Self {
        Self {
            raw: (self.vtb.clone)(self.raw),
            vtb: self.vtb,
        }
    }
}

impl Drop for AnyArc {
    fn drop(&mut self) {
        (self.vtb.drop)(self.raw)
    }
}

#[derive(Clone)]
pub struct Facade {
    map: Arc<RwLock<BTreeMap<ConstTypeId, AnyArc>>>,
}

impl Facade {
    pub fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
        let map_mut = self.map.read().expect("facade lock poisoned");
        match map_mut.get(&ConstTypeId::of::<T>()) {
            None => Err(anyhow!("unexpected type")),
            Some(e) => Ok(e.clone().recover::<T>()),
        }
    }

    pub fn empty() -> Self {
        FacadeBuilder::new().build()
    }
}

pub struct FacadeBuilder {
    map: BTreeMap<ConstTypeId, AnyArc>,
}

impl Default for FacadeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FacadeBuilder {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn add<T: Send + Sync + 'static>(&mut self, v: T) -> anyhow::Result<()> {
        if self.map.contains_key(&ConstTypeId::of::<T>()) {
            bail!("FacadeBuilder::add key already exists");
        }
        self.map
            .insert(ConstTypeId::of::<T>(), AnyArc::new(Arc::new(v)));
        Ok(())
    }

    pub fn share<T: Send + Sync + 'static>(&mut self, v: Arc<T>) -> anyhow::Result<()> {
        if self.map.contains_key(&ConstTypeId::of::<T>()) {
            bail!("FacadeBuilder::add key already exists");
        }
        self.map.insert(ConstTypeId::of::<T>(), AnyArc::new(v));
        Ok(())
    }

    pub fn build(self) -> Facade {
        Facade {
            map: Arc::new(RwLock::new(self.map)),
        }
    }
}
