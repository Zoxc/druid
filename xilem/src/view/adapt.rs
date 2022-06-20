// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{any::Any, marker::PhantomData};

use crate::{event::EventResult, id::Id};

use super::{Cx, View, ViewState};

pub struct Adapt<
    T,
    A,
    U,
    B,
    F: Fn(&mut T, AdaptThunk<U, B, C>) -> AdaptThunkResult<A, C>,
    C: ViewState,
> {
    f: F,
    child: C,
    phantom: PhantomData<fn() -> (T, A, U, B)>,
}

/// A "thunk" which dispatches an event to an adapt node's child.
///
/// The closure passed to Adapt should call this thunk with the child's
/// app state.
pub struct AdaptThunk<'a, U, B, C: ViewState> {
    call: &'a mut dyn FnMut(&mut U) -> AdaptThunkResult<B, C>,
}

pub enum AdaptThunkResult<A, C: ViewState> {
    Build((Id, C::State, C::Element)),
    Rebuild(bool),
    Event(EventResult<A>),
}

impl<T, A, U, B, F: Fn(&mut T, AdaptThunk<U, B, C>) -> AdaptThunkResult<A, C>, C: View<U, B>>
    Adapt<T, A, U, B, F, C>
{
    pub fn new(f: F, child: C) -> Self {
        Adapt {
            f,
            child,
            phantom: Default::default(),
        }
    }
}

impl<'a, U, B, C: View<U, B>> AdaptThunk<'a, U, B, C> {
    pub fn call(self, app_state: &mut U) -> AdaptThunkResult<B, C> {
        (self.call)(app_state)
    }
}

impl<
        T,
        A,
        U,
        B,
        F: Fn(&mut T, AdaptThunk<U, B, C>) -> AdaptThunkResult<A, C> + Send,
        C: ViewState,
    > ViewState for Adapt<T, A, U, B, F, C>
{
    type State = C::State;

    type Element = C::Element;
}

impl<
        T,
        A,
        U,
        B,
        F: Fn(&mut T, AdaptThunk<U, B, C>) -> AdaptThunkResult<A, C> + Send,
        C: View<U, B>,
    > View<T, A> for Adapt<T, A, U, B, F, C>
{
    fn build(&self, cx: &mut Cx, app_state: &mut T) -> (Id, Self::State, Self::Element) {
        let call =
            &mut |app_state: &mut U| AdaptThunkResult::Build(self.child.build(cx, app_state));
        match (self.f)(app_state, AdaptThunk { call }) {
            AdaptThunkResult::Build(value) => value,
            _ => panic!(),
        }
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
        app_state: &mut T,
    ) -> bool {
        let call = &mut |app_state: &mut U| {
            AdaptThunkResult::Rebuild(self.child.rebuild(
                cx,
                &prev.child,
                id,
                state,
                element,
                app_state,
            ))
        };
        match (self.f)(app_state, AdaptThunk { call }) {
            AdaptThunkResult::Rebuild(value) => value,
            _ => panic!(),
        }
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        let mut event = Some(event);
        let call = &mut |app_state: &mut U| {
            AdaptThunkResult::Event(self.child.event(
                id_path,
                state,
                event.take().unwrap(),
                app_state,
            ))
        };
        match (self.f)(app_state, AdaptThunk { call }) {
            AdaptThunkResult::Event(value) => value,
            _ => panic!(),
        }
    }
}
