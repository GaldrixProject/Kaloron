// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Internal composite field and tuple builders for generated code.
//!
//! These types are used by the `#[derive(TypeShape)]` macro expansion to
//! construct the send/recv field dispatch machinery. They are `#[doc(hidden)]`
//! and should not be used directly.

#![allow(unused, private_bounds, clippy::type_complexity)]

use crate::*;
use core::marker::PhantomData;
use seq_macro::seq;

trait __TK: Sized {
    type Solid: TypeShape;

    fn check(&self, a: ActivationState) -> anyhow::Result<()>;

    fn visit_send(&self, v: &mut impl SendVisitor) -> anyhow::Result<()>;

    fn visit_recv(o: &mut Option<Self>, v: &mut impl RecvVisitor) -> anyhow::Result<()>;
}

impl<T: TypeShape> __TK for Gated<T> {
    type Solid = T;

    fn check(&self, a: ActivationState) -> anyhow::Result<()> {
        match a {
            ActivationState::Active => {
                if self.is_active() {
                    Ok(())
                } else {
                    anyhow::bail!("expected active, got inactive")
                }
            }
            ActivationState::Deprecated => Ok(()),
            ActivationState::Inactive => {
                if self.is_void() {
                    Ok(())
                } else {
                    anyhow::bail!("expected inactive, got active")
                }
            }
        }
    }

    fn visit_send(&self, v: &mut impl SendVisitor) -> anyhow::Result<()> {
        match self {
            Gated::Active(it) => v.visit(it),
            Gated::Void => Ok(()),
        }
    }

    fn visit_recv(o: &mut Option<Self>, v: &mut impl RecvVisitor) -> anyhow::Result<()> {
        *o = Some(Some(v.visit::<T>()?).into());
        Ok(())
    }
}

impl<T> __TK for T
where
    T: TypeShape,
{
    type Solid = T;

    fn check(&self, a: ActivationState) -> anyhow::Result<()> {
        Ok(())
    }

    fn visit_send(&self, v: &mut impl SendVisitor) -> anyhow::Result<()> {
        v.visit(self)
    }

    fn visit_recv(o: &mut Option<Self>, v: &mut impl RecvVisitor) -> anyhow::Result<()> {
        *o = Some(v.visit()?);
        Ok(())
    }
}

fn __tk<T: __TK>(o: &T, a: ActivationState) -> anyhow::Result<()> {
    o.check(a)
}

trait __Tc {
    const ITEMS: isize;
    const LEVEL: isize;
}

macro_rules! __impl16 {
    ($Trait:ident for $Type:ident where B=$B:ident { $($body:tt)* }) => {
        impl<
            T0: $B, T1: $B, T2: $B, T3: $B, T4: $B, T5: $B, T6: $B, T7: $B,
            T8: $B, T9: $B, T10: $B, T11: $B, T12: $B, T13: $B, T14: $B, T15: $B,
        > $Trait for $Type<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
        {
            $($body)*
        }
    };
}

const fn il_v(l0: isize, i: isize, l: isize, bd: &mut bool) {
    if *bd {
        assert!(i == 0);
    } else {
        assert!(l <= l0);
        if i < (1 << (l0 * 4)) {
            *bd = true
        }
    }
}

seq!(N in 0..=15 { pub struct __I16< #(T~N: __Tc,)* >( PhantomData<( #(T~N,)* )>); });

__impl16!(__Tc for __I16 where B=__Tc {
    const ITEMS: isize = T0::ITEMS
        + T1::ITEMS
        + T2::ITEMS
        + T3::ITEMS
        + T4::ITEMS
        + T5::ITEMS
        + T6::ITEMS
        + T7::ITEMS
        + T8::ITEMS
        + T9::ITEMS
        + T10::ITEMS
        + T11::ITEMS
        + T12::ITEMS
        + T13::ITEMS
        + T14::ITEMS
        + T15::ITEMS;

    const LEVEL: isize = {
        let l0 = T0::LEVEL;
        let mut bd = false;
        il_v(l0, T0::ITEMS, T0::LEVEL, &mut bd);
        il_v(l0, T1::ITEMS, T1::LEVEL, &mut bd);
        il_v(l0, T2::ITEMS, T2::LEVEL, &mut bd);
        il_v(l0, T3::ITEMS, T3::LEVEL, &mut bd);
        il_v(l0, T4::ITEMS, T4::LEVEL, &mut bd);
        il_v(l0, T5::ITEMS, T5::LEVEL, &mut bd);
        il_v(l0, T6::ITEMS, T6::LEVEL, &mut bd);
        il_v(l0, T7::ITEMS, T7::LEVEL, &mut bd);
        il_v(l0, T8::ITEMS, T8::LEVEL, &mut bd);
        il_v(l0, T9::ITEMS, T9::LEVEL, &mut bd);
        il_v(l0, T10::ITEMS, T10::LEVEL, &mut bd);
        il_v(l0, T11::ITEMS, T11::LEVEL, &mut bd);
        il_v(l0, T12::ITEMS, T12::LEVEL, &mut bd);
        il_v(l0, T13::ITEMS, T13::LEVEL, &mut bd);
        il_v(l0, T14::ITEMS, T14::LEVEL, &mut bd);
        il_v(l0, T15::ITEMS, T15::LEVEL, &mut bd);
        T0::LEVEL
    };
});

#[inline(never)]
fn tb_r() -> anyhow::Result<()> {
    anyhow::bail!("visit index out of bounds while building data structure")
}

#[inline(never)]
fn tb_b() -> anyhow::Result<()> {
    anyhow::bail!("incomplete data structure: not all fields were visited")
}

#[inline(never)]
fn tv_r() -> anyhow::Result<()> {
    anyhow::bail!("visit index out of bounds while reading data structure")
}

fn tb_g<T>(o: &mut Option<T>) -> T {
    unsafe { o.take().unwrap_unchecked() }
}

trait __TbC: Default + TupleRecv + __Tc {
    fn check(&self) -> anyhow::Result<()>;
    fn validate_recv(&self, a: &[ActivationState]) -> anyhow::Result<()>;
}

trait __TvC: TupleSend + __Tc {
    fn validate_send(&self, a: &[ActivationState]) -> anyhow::Result<()>;
}

seq!(N in 01..=32 {
    #(
        seq!(I in 0..N {
            pub struct __Tb~N< #(T~I,)* >( #(Option<T~I>,)* );

            impl< #(T~I,)* > Default for __Tb~N< #(T~I,)* > {
                fn default() -> Self {
                    Self( #(None,)* )
                }
            }

            impl< #(T~I,)* > __Tc for __Tb~N< #(T~I,)* > {
                const ITEMS: isize = N;
                const LEVEL: isize = 1;
            }

            impl< #(T~I: __TK,)* > __TbC for __Tb~N< #(T~I,)* > {
                #[allow(clippy::nonminimal_bool)]
                fn check(&self) -> anyhow::Result<()> {
                    let ok = true #( && self.I.is_some() )*;
                    if ok { Ok(()) } else { tb_b() }
                }

                fn validate_recv(&self, a: &[ActivationState]) -> anyhow::Result<()> {
                    #(__tbk(self.I.as_ref(), a[I])?;)*
                    Ok(())
                }
            }

            impl< #(T~I: __TK,)* > TupleRecv for __Tb~N< #(T~I,)* > {
                fn visit(&mut self, i: isize, v: &mut impl RecvVisitor) -> anyhow::Result<()> {
                    match i {
                        #(I => T~I::visit_recv(&mut self.I, v),)*
                        _ => tb_r(),
                    }
                }
            }

            impl< #(T~I,)* > __Tb~N< #(T~I,)* > {
                #(pub fn g~I(&mut self) -> T~I { tb_g(&mut self.I) })*
                #(pub fn s~I(&mut self, v: T~I) { self.I = Some(v); })*
                #(pub fn h~I(&self) -> bool { self.I.is_some() })*
            }
        });
    )*
});

seq!(N in 01..=16 {
    #(
        seq!(I in 0..N {
            pub struct __Tv~N<'a, #(T~I,)* >( #(pub &'a T~I,)* );

            impl<'a, #(T~I,)* > __Tc for __Tv~N<'a, #(T~I,)* > {
                const ITEMS: isize = N;
                const LEVEL: isize = 1;
            }

            impl<'a, #(T~I: __TK,)* > __TvC for __Tv~N<'a, #(T~I,)* > {
                fn validate_send(&self, a: &[ActivationState]) -> anyhow::Result<()> {
                    #(__tk(self.I, a[I])?;)*
                    Ok(())
                }
            }

            impl<'a, #(T~I: __TK,)* > TupleSend for __Tv~N<'a, #(T~I,)* > {
                fn visit(&self, i: isize, v: &mut impl SendVisitor) -> anyhow::Result<()> {
                    match i {
                        #(I => T~I::visit_send(self.I, v),)*
                        _ => tv_r(),
                    }
                }
            }
        });
    )*
});

pub struct __Tb0;

impl Default for __Tb0 {
    fn default() -> Self {
        Self
    }
}

impl __Tc for __Tb0 {
    const ITEMS: isize = 0;
    const LEVEL: isize = 0;
}

impl __TbC for __Tb0 {
    fn check(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate_recv(&self, _a: &[ActivationState]) -> anyhow::Result<()> {
        Ok(())
    }
}

impl TupleRecv for __Tb0 {
    fn visit(&mut self, i: isize, v: &mut impl RecvVisitor) -> anyhow::Result<()> {
        if i == 0 { Ok(()) } else { tb_r() }
    }
}

pub struct __Tv0;

impl __Tc for __Tv0 {
    const ITEMS: isize = 0;
    const LEVEL: isize = 0;
}

impl __TvC for __Tv0 {
    fn validate_send(&self, _a: &[ActivationState]) -> anyhow::Result<()> {
        Ok(())
    }
}

impl TupleSend for __Tv0 {
    fn visit(&self, i: isize, v: &mut impl SendVisitor) -> anyhow::Result<()> {
        if i == 0 { Ok(()) } else { tv_r() }
    }
}

seq!(N in 0..=15 { pub struct __TbCp< #(T~N: __TbC,)* >( #(pub T~N,)* ); });

__impl16!(Default for __TbCp where B=__TbC {
    fn default() -> Self {
        Self(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }
});

__impl16!(__Tc for __TbCp where B=__TbC {
    const ITEMS: isize = __I16::<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>::ITEMS;
    const LEVEL: isize = __I16::<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>::LEVEL;
});

__impl16!(__TbC for __TbCp where B=__TbC {
    fn check(&self) -> anyhow::Result<()> {
        self.0.check()?;
        self.1.check()?;
        self.2.check()?;
        self.3.check()?;
        self.4.check()?;
        self.5.check()?;
        self.6.check()?;
        self.7.check()?;
        self.8.check()?;
        self.9.check()?;
        self.10.check()?;
        self.11.check()?;
        self.12.check()?;
        self.13.check()?;
        self.14.check()?;
        self.15.check()
    }

    fn validate_recv(&self, a: &[ActivationState]) -> anyhow::Result<()> {
        let mut off = 0usize;
        let n = T0::ITEMS as usize; self.0.validate_recv(&a[off..off + n])?; off += n;
        let n = T1::ITEMS as usize; self.1.validate_recv(&a[off..off + n])?; off += n;
        let n = T2::ITEMS as usize; self.2.validate_recv(&a[off..off + n])?; off += n;
        let n = T3::ITEMS as usize; self.3.validate_recv(&a[off..off + n])?; off += n;
        let n = T4::ITEMS as usize; self.4.validate_recv(&a[off..off + n])?; off += n;
        let n = T5::ITEMS as usize; self.5.validate_recv(&a[off..off + n])?; off += n;
        let n = T6::ITEMS as usize; self.6.validate_recv(&a[off..off + n])?; off += n;
        let n = T7::ITEMS as usize; self.7.validate_recv(&a[off..off + n])?; off += n;
        let n = T8::ITEMS as usize; self.8.validate_recv(&a[off..off + n])?; off += n;
        let n = T9::ITEMS as usize; self.9.validate_recv(&a[off..off + n])?; off += n;
        let n = T10::ITEMS as usize; self.10.validate_recv(&a[off..off + n])?; off += n;
        let n = T11::ITEMS as usize; self.11.validate_recv(&a[off..off + n])?; off += n;
        let n = T12::ITEMS as usize; self.12.validate_recv(&a[off..off + n])?; off += n;
        let n = T13::ITEMS as usize; self.13.validate_recv(&a[off..off + n])?; off += n;
        let n = T14::ITEMS as usize; self.14.validate_recv(&a[off..off + n])?; off += n;
        let n = T15::ITEMS as usize; self.15.validate_recv(&a[off..off + n])?;
        Ok(())
    }
});

__impl16!(TupleRecv for __TbCp where B=__TbC {
    fn visit(&mut self, i: isize, v: &mut impl RecvVisitor) -> anyhow::Result<()> {
        if (i as usize) >= Self::ITEMS as usize {
            return tb_r();
        }
        let l = i >> ((Self::LEVEL) * 4);
        let r = i & ((1 << ((Self::LEVEL) * 4)) - 1);
        match l {
            0 => self.0.visit(r, v),
            1 => self.1.visit(r, v),
            2 => self.2.visit(r, v),
            3 => self.3.visit(r, v),
            4 => self.4.visit(r, v),
            5 => self.5.visit(r, v),
            6 => self.6.visit(r, v),
            7 => self.7.visit(r, v),
            8 => self.8.visit(r, v),
            9 => self.9.visit(r, v),
            10 => self.10.visit(r, v),
            11 => self.11.visit(r, v),
            12 => self.12.visit(r, v),
            13 => self.13.visit(r, v),
            14 => self.14.visit(r, v),
            15 => self.15.visit(r, v),
            _ => tb_r(),
        }
    }
});

seq!(N in 0..=15 { pub struct __TvCp< #(T~N: __TvC,)* >( #(pub T~N,)* ); });

__impl16!(__Tc for __TvCp where B=__TvC {
    const ITEMS: isize = __I16::<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>::ITEMS;
    const LEVEL: isize = __I16::<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>::LEVEL;
});

__impl16!(__TvC for __TvCp where B=__TvC {
    fn validate_send(&self, a: &[ActivationState]) -> anyhow::Result<()> {
        let mut off = 0usize;
        let n = T0::ITEMS as usize; self.0.validate_send(&a[off..off + n])?; off += n;
        let n = T1::ITEMS as usize; self.1.validate_send(&a[off..off + n])?; off += n;
        let n = T2::ITEMS as usize; self.2.validate_send(&a[off..off + n])?; off += n;
        let n = T3::ITEMS as usize; self.3.validate_send(&a[off..off + n])?; off += n;
        let n = T4::ITEMS as usize; self.4.validate_send(&a[off..off + n])?; off += n;
        let n = T5::ITEMS as usize; self.5.validate_send(&a[off..off + n])?; off += n;
        let n = T6::ITEMS as usize; self.6.validate_send(&a[off..off + n])?; off += n;
        let n = T7::ITEMS as usize; self.7.validate_send(&a[off..off + n])?; off += n;
        let n = T8::ITEMS as usize; self.8.validate_send(&a[off..off + n])?; off += n;
        let n = T9::ITEMS as usize; self.9.validate_send(&a[off..off + n])?; off += n;
        let n = T10::ITEMS as usize; self.10.validate_send(&a[off..off + n])?; off += n;
        let n = T11::ITEMS as usize; self.11.validate_send(&a[off..off + n])?; off += n;
        let n = T12::ITEMS as usize; self.12.validate_send(&a[off..off + n])?; off += n;
        let n = T13::ITEMS as usize; self.13.validate_send(&a[off..off + n])?; off += n;
        let n = T14::ITEMS as usize; self.14.validate_send(&a[off..off + n])?; off += n;
        let n = T15::ITEMS as usize; self.15.validate_send(&a[off..off + n])?;
        Ok(())
    }
});

__impl16!(TupleSend for __TvCp where B=__TvC {
    fn visit(&self, i: isize, v: &mut impl SendVisitor) -> anyhow::Result<()> {
        if (i as usize) >= Self::ITEMS as usize {
            return tb_r();
        }
        let l = i >> ((Self::LEVEL) * 4);
        let r = i & ((1 << ((Self::LEVEL) * 4)) - 1);
        match l {
            0 => self.0.visit(r, v),
            1 => self.1.visit(r, v),
            2 => self.2.visit(r, v),
            3 => self.3.visit(r, v),
            4 => self.4.visit(r, v),
            5 => self.5.visit(r, v),
            6 => self.6.visit(r, v),
            7 => self.7.visit(r, v),
            8 => self.8.visit(r, v),
            9 => self.9.visit(r, v),
            10 => self.10.visit(r, v),
            11 => self.11.visit(r, v),
            12 => self.12.visit(r, v),
            13 => self.13.visit(r, v),
            14 => self.14.visit(r, v),
            15 => self.15.visit(r, v),
            _ => tv_r(),
        }
    }
});

#[inline(always)]
fn __tbk<T: __TK>(o: Option<&T>, a: ActivationState) -> anyhow::Result<()> {
    if let Some(v) = o {
        __tk(v, a)?;
    }
    Ok(())
}

#[inline(always)]
pub fn __tbc(s: &impl __TbC) -> anyhow::Result<()> {
    s.check()
}

#[inline(always)]
pub fn __tbv(s: &mut impl __TbC, i: isize, v: &mut impl RecvVisitor) -> anyhow::Result<()> {
    s.visit(i, v)
}

#[inline(always)]
pub fn __tvv(s: &impl __TvC, i: isize, v: &mut impl SendVisitor) -> anyhow::Result<()> {
    s.visit(i, v)
}

#[inline(always)]
pub fn __tbac(s: &impl __TbC, a: &[ActivationState]) -> anyhow::Result<()> {
    s.validate_recv(a)
}

#[inline(always)]
pub fn __tvac(s: &impl __TvC, a: &[ActivationState]) -> anyhow::Result<()> {
    s.validate_send(a)
}

impl<'a, T: TypeShape> NewTypeSend for __Tv01<'a, T> {
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()> {
        __tvv(self, 0, visitor)
    }
}

impl<T: TypeShape> NewTypeRecv for __Tb01<T> {
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
        __tbv(self, 0, visitor)
    }
}
