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

use std::{any::Any, future::Future, marker::PhantomData, pin::Pin};

use futures::future::{AbortHandle, Abortable, Aborted};
use futures_task::{Context, Poll, Waker};
use tokio::task::JoinHandle;

use crate::{event::EventResult, id::Id, widget::AnyWidget, ViewState};

use super::{Cx, View};

pub struct Loader<T, A, V, P, FF, F: Fn() -> FF> {
    pending: P,
    callback: F,
    phantom: PhantomData<fn() -> (T, A, V)>,
}

pub enum LoaderState<V: ViewState, P: ViewState> {
    Pending {
        state: P::State,
        task: JoinHandle<Result<V, Aborted>>,
        waker: Waker,
        abort_handle: AbortHandle,
    },
    Build(Option<V>),
    Complete(V, V::State),
}

impl<V: ViewState, P: ViewState> Drop for LoaderState<V, P> {
    fn drop(&mut self) {
        match self {
            Self::Pending {
                state: _,
                task: _,
                waker: _,
                abort_handle,
            } => {
                abort_handle.abort();
            }
            _ => (),
        }
    }
}

pub fn loader<T, A, V, P, FF, F: Fn() -> FF>(pending: P, callback: F) -> Loader<T, A, V, P, FF, F> {
    Loader::new(pending, callback)
}

impl<T, A, V, P, FF, F: Fn() -> FF> Loader<T, A, V, P, FF, F> {
    pub fn new(pending: P, callback: F) -> Self {
        Loader {
            pending,
            callback,
            phantom: Default::default(),
        }
    }
}

impl<T, A, V: ViewState, P: ViewState, FF, F: Fn() -> FF + Send> ViewState
    for Loader<T, A, V, P, FF, F>
where
    FF: Future<Output = V> + Send + 'static,
    V: 'static,
    V::Element: 'static,
    P: 'static,
    P::Element: 'static,
{
    type State = LoaderState<V, P>;

    type Element = Box<dyn AnyWidget>;
}

impl<T, A, V: View<T, A>, P: View<T, A>, FF, F: Fn() -> FF + Send> View<T, A>
    for Loader<T, A, V, P, FF, F>
where
    FF: Future<Output = V> + Send + 'static,
    V: 'static,
    V::Element: 'static,
    P: 'static,
    P::Element: 'static,
{
    fn build(&self, cx: &mut Cx, app_state: &mut T) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let future = (self.callback)();
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let future = Abortable::new(future, abort_registration);
            let mut task = tokio::spawn(future);
            let waker = cx.waker();

            let mut future_cx = Context::from_waker(&waker);
            match Pin::new(&mut task).poll(&mut future_cx) {
                Poll::Ready(v) => {
                    let view = v.unwrap().unwrap();
                    let (_, state, element) = view.build(cx, app_state);
                    let element: Box<dyn AnyWidget> = Box::new(element);
                    (LoaderState::Complete(view, state), element)
                }
                Poll::Pending => {
                    cx.add_pending_async(cx.id_path().last().copied().unwrap());
                    let (_, state, element) = self.pending.build(cx, app_state);
                    let element: Box<dyn AnyWidget> = Box::new(element);
                    (
                        LoaderState::Pending {
                            state,
                            task,
                            waker,
                            abort_handle,
                        },
                        element,
                    )
                }
            }
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
        _app_state: &mut T,
    ) -> bool {
        match state {
            LoaderState::Build(view) => {
                let view = view.take().unwrap();
                // How do we know that the future result did not change here?
                let (_, new_state, new_element) = view.build(cx, _app_state);
                *state = LoaderState::Complete(view, new_state);
                *element = Box::new(new_element);
            }
            _ => {
                // We cannot compare the callback so we must always rebuild.
                let (new_id, new_state, new_element) = self.build(cx, _app_state);
                *id = new_id;
                *state = new_state;
                *element = new_element;

                // How to rebuild the completed element? I guess we do it after the future completes.
                // Would it always get built anew anyway?
            }
        }
        true
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        match state {
            LoaderState::Pending {
                state: pending_state,
                task,
                waker,
                abort_handle: _,
            } => {
                if id_path.is_empty() {
                    let mut future_cx = Context::from_waker(&waker);
                    match Pin::new(task).poll(&mut future_cx) {
                        Poll::Ready(v) => {
                            *state = LoaderState::Build(Some(v.unwrap().unwrap()));
                            EventResult::RequestRebuild
                        }
                        Poll::Pending => EventResult::Stale,
                    }
                } else {
                    self.pending
                        .event(&id_path[1..], pending_state, event, app_state)
                }
            }
            LoaderState::Build(..) => EventResult::Stale,
            LoaderState::Complete(view, state) => {
                view.event(&id_path[1..], state, event, app_state)
            }
        }
    }
}
