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

/// Calls `build_future` and runs the future it returns. It will display `pending` until the future returns.
/// `then` will be called when the future returns and that produces the final view to be displayed.
pub fn async_then<
    Data,
    T,
    A,
    ThenView,
    PendingView,
    Task: Future,
    BuildFuture: Fn(&Data) -> Task,
    Then: Fn(&mut T, &Task::Output) -> ThenView,
>(
    pending: PendingView,
    data: Data,
    build_future: BuildFuture,
    then: Then,
) -> AsyncThen<Data, T, A, ThenView, PendingView, Task, BuildFuture, Then> {
    AsyncThen::new(pending, data, build_future, then)
}

pub struct AsyncThen<
    Data,
    T,
    A,
    ThenView,
    PendingView,
    Task: Future,
    BuildFuture: Fn(&Data) -> Task,
    Then: Fn(&mut T, &Task::Output) -> ThenView,
> {
    data: Data,
    pending: PendingView,
    build_future: BuildFuture,
    then: Then,
    phantom: PhantomData<fn() -> (T, A, ThenView)>,
}

pub enum AsyncThenState<ThenView: ViewState, PendingView: ViewState, TaskOutput> {
    Pending {
        id: Id,
        state: PendingView::State,
        task: JoinHandle<Result<TaskOutput, Aborted>>,
        waker: Waker,
        abort_handle: AbortHandle,
    },
    Build(Option<TaskOutput>),
    Complete {
        id: Id,
        output: TaskOutput,
        view: ThenView,
        state: ThenView::State,
    },
}

impl<ThenView: ViewState, PendingView: ViewState, TaskOutput> Drop
    for AsyncThenState<ThenView, PendingView, TaskOutput>
{
    fn drop(&mut self) {
        match self {
            Self::Pending {
                id: _,
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

impl<
        Data,
        T,
        A,
        ThenView,
        PendingView,
        Task: Future,
        BuildFuture: Fn(&Data) -> Task,
        Then: Fn(&mut T, &Task::Output) -> ThenView,
    > AsyncThen<Data, T, A, ThenView, PendingView, Task, BuildFuture, Then>
{
    pub fn new(pending: PendingView, data: Data, build_future: BuildFuture, then: Then) -> Self {
        AsyncThen {
            data,
            pending,
            build_future,
            then,
            phantom: Default::default(),
        }
    }
}

impl<Data, T, A, ThenView, PendingView, Task, BuildFuture, Then> ViewState
    for AsyncThen<Data, T, A, ThenView, PendingView, Task, BuildFuture, Then>
where
    Data: PartialEq + Send,
    BuildFuture: Fn(&Data) -> Task + Send,
    Then: Fn(&mut T, &Task::Output) -> ThenView + Send,
    Task: Future<Output = ThenView> + Send + 'static,
    ThenView: ViewState + 'static,
    ThenView::Element: 'static,
    PendingView: ViewState + 'static,
    PendingView::Element: 'static,
{
    type State = AsyncThenState<ThenView, PendingView, Task::Output>;

    type Element = Box<dyn AnyWidget>;
}

impl<Data, T, A, ThenView, PendingView, Task, BuildFuture, Then> View<T, A>
    for AsyncThen<Data, T, A, ThenView, PendingView, Task, BuildFuture, Then>
where
    Data: PartialEq + Send,
    BuildFuture: Fn(&Data) -> Task + Send,
    Then: Fn(&mut T, &Task::Output) -> ThenView + Send,
    Task: Future<Output = ThenView> + Send + 'static,
    ThenView: View<T, A> + 'static,
    ThenView::Element: 'static,
    PendingView: View<T, A> + 'static,
    PendingView::Element: 'static,
{
    fn build(&self, cx: &mut Cx, app_state: &mut T) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let future = (self.build_future)(&self.data);
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let future = Abortable::new(future, abort_registration);
            let mut task = tokio::spawn(future);
            let waker = cx.waker();

            let mut future_cx = Context::from_waker(&waker);
            match Pin::new(&mut task).poll(&mut future_cx) {
                Poll::Ready(output) => {
                    let output = output.unwrap().unwrap();
                    let view = (self.then)(app_state, &output);
                    let (id, state, element) = view.build(cx, app_state);
                    let element: Box<dyn AnyWidget> = Box::new(element);
                    (
                        AsyncThenState::Complete {
                            id,
                            view,
                            state,
                            output,
                        },
                        element,
                    )
                }
                Poll::Pending => {
                    cx.add_pending_async(cx.id_path().last().copied().unwrap());
                    let (id, state, element) = self.pending.build(cx, app_state);
                    let element: Box<dyn AnyWidget> = Box::new(element);
                    (
                        AsyncThenState::Pending {
                            id,
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
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
        app_state: &mut T,
    ) -> bool {
        if prev.data != self.data {
            let (new_id, new_state, new_element) = self.build(cx, app_state);
            *id = new_id;
            *state = new_state;
            *element = new_element;

            // FIXME: Reuse pending? Or should it reset things as counters?
            true
        } else {
            match state {
                AsyncThenState::Build(output) => {
                    let output = output.take().unwrap();
                    let view = (self.then)(app_state, &output);
                    let (id, new_state, new_element) = view.build(cx, app_state);
                    *state = AsyncThenState::Complete {
                        view,
                        state: new_state,
                        id,
                        output,
                    };
                    *element = Box::new(new_element);
                    true
                }
                AsyncThenState::Pending { id, state, .. } => self.pending.rebuild(
                    cx,
                    &prev.pending,
                    id,
                    state,
                    (**element).as_any_mut().downcast_mut().unwrap(),
                    app_state,
                ),
                AsyncThenState::Complete {
                    id,
                    view,
                    output,
                    state,
                } => {
                    let new_view = (self.then)(app_state, output);
                    let changed = new_view.rebuild(
                        cx,
                        view,
                        id,
                        state,
                        (**element).as_any_mut().downcast_mut().unwrap(),
                        app_state,
                    );
                    *view = new_view;
                    changed
                }
            }
        }
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        match state {
            AsyncThenState::Pending {
                state: pending_state,
                task,
                waker,
                ..
            } => {
                if id_path.is_empty() {
                    let mut future_cx = Context::from_waker(&waker);
                    match Pin::new(task).poll(&mut future_cx) {
                        Poll::Ready(output) => {
                            *state = AsyncThenState::Build(Some(output.unwrap().unwrap()));
                            EventResult::RequestRebuild
                        }
                        Poll::Pending => EventResult::Stale,
                    }
                } else {
                    self.pending
                        .event(&id_path[1..], pending_state, event, app_state)
                }
            }
            AsyncThenState::Build(..) => EventResult::Stale,
            AsyncThenState::Complete { view, state, .. } => {
                view.event(&id_path[1..], state, event, app_state)
            }
        }
    }
}
