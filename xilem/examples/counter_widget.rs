use druid_shell::kurbo::Size;
use std::any::Any;
use std::fmt::Debug;
use xilem::widget::align::SingleAlignment;
use xilem::widget::AlignCx;
use xilem::{button, v_stack, App, AppLauncher, View};
use xilem::{
    widget::{
        self, text::TextWidget, EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod, RawEvent,
        UpdateCx,
    },
    Cx, EventResult, Id, Widget,
};

pub fn both_as_widget() -> impl Widget {
    widget::vstack::VStack::new({
        let reactive_counter = Pod::new(ViewAsWidget::new(
            ReactiveCounter::default(),
            reactive_counter,
        ));
        let sep = Pod::new(TextWidget::new("--------------------------".to_string()));
        let counter = Pod::new(CounterWidget::new());
        vec![reactive_counter, sep, counter]
    })
}

fn app(_: &mut ()) -> impl View<()> {
    v_stack((
        "Counter demo".to_string(),
        "--------------------------".to_string(),
        WidgetAsView(both_as_widget),
    ))
}

pub fn main() {
    let app = App::new((), app);
    AppLauncher::new(app).run();
}

// Traditional GUI style counter
struct CounterWidget {
    count: u32,
    stack: widget::vstack::VStack,
}

impl CounterWidget {
    fn new() -> Self {
        let widgets = vec![
            Pod::new(TextWidget::new(format!("Count: {}", 0))),
            Pod::new(widget::button::Button::new(
                &Vec::new(),
                "Increase".to_string(),
            )),
        ];
        CounterWidget {
            count: 0,
            stack: widget::vstack::VStack::new(widgets),
        }
    }

    fn click(&mut self) {
        self.count += 1;

        let label = &mut self.stack.children_mut()[0];
        label.request_update();
        let label: &mut TextWidget = label.downcast_mut().unwrap();
        label.set_text(format!("Count: {}", self.count));
    }
}

impl Widget for CounterWidget {
    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        let mut events = Vec::new();
        self.stack
            .event(&mut cx.with_event_sink(&mut events), event);
        if !events.is_empty() {
            self.click();
            cx.request_update();
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.stack.lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.stack.update(cx);
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        self.stack.prelayout(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        self.stack.layout(cx, proposed_size)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.stack.paint(cx);
    }
}

// Reactive counter
#[derive(Default, Debug)]
struct ReactiveCounter {
    count: u32,
}

fn reactive_counter(data: &mut ReactiveCounter) -> impl View<ReactiveCounter> {
    v_stack((
        format!("Reactive Count: {}", data.count),
        button("Increase".to_string(), |data: &mut ReactiveCounter| {
            data.count += 1
        }),
    ))
}

// Use a view as a widget
struct ViewAsWidget<S, V: View<S>> {
    logic_state: S,
    view_tree_builder: fn(&mut S) -> V,
    view_state: V::State,
    view: V,
    element: Pod,
    id: Id,
    rebuild_need: bool,
    cx: Cx,
}

impl<S, V: View<S>> ViewAsWidget<S, V>
where
    V::Element: Widget + 'static,
{
    fn new(mut logic_state: S, view_tree_builder: fn(&mut S) -> V) -> Self {
        let mut cx = Cx::new();
        let view = view_tree_builder(&mut logic_state);
        let (id, view_state, element) = view.build(&mut cx);
        let element = Pod::new(element);
        ViewAsWidget {
            view_tree_builder,
            view,
            view_state,
            logic_state,
            id,
            element,
            rebuild_need: false,
            cx,
        }
    }
}
/*
fn id(&self) -> Option<Id> {
    Some(self.widget_id)
}

fn message(&mut self, id_path: &[Id], event: Box<dyn Any>) -> EventResult<()> {
    self.view.event(
        &id_path[1..],
        &mut self.view_state,
        self.element.downcast_mut().unwrap(),
        event,
        &mut self.logic_state,
    );

    // We need to always rebuild in case the event changed some state
    self.element.request_update();
    self.rebuild_need = true;
    EventResult::RequestRebuild
}
 */
impl<S, V: View<S>> Widget for ViewAsWidget<S, V>
where
    V::Element: Widget + 'static,
{
    fn update(&mut self, cx: &mut UpdateCx) {
        if self.rebuild_need {
            self.rebuild_need = false;

            let view = (self.view_tree_builder)(&mut self.logic_state);
            let changed = view.rebuild(
                &mut self.cx,
                &self.view,
                &mut self.id,
                &mut self.view_state,
                self.element.downcast_mut().unwrap(),
            );

            self.view = view;

            if changed {
                self.element.update(cx);
            }
        }
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        let mut events = Vec::new();
        self.element
            .event(&mut cx.with_event_sink(&mut events), event);

        for event in events {
            self.view.event(
                &event.id_path[1..],
                &mut self.view_state,
                event.body,
                &mut self.logic_state,
            );
        }

        // We need to always rebuild in case the event changed some state
        self.rebuild_need = true;
        self.element.request_update();
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        self.element.prelayout(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        self.element.layout(cx, proposed_size)
    }

    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        self.element.align(cx, alignment);
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.element.paint(cx);
    }
}

// Use a widget as a view
struct WidgetAsView<T>(fn() -> T);

impl<T: Widget> View<(), ()> for WidgetAsView<T> {
    type State = bool;

    type Element = T;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| self.0());
        (id, false, element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        state: &mut Self::State,
        _element: &mut Self::Element,
    ) -> bool {
        std::mem::take(state)
    }

    fn event(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _event: Box<dyn Any>,
        _app_state: &mut (),
    ) -> EventResult<()> {
        panic!()
    }
}
