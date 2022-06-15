use std::any::Any;

use druid_shell::kurbo::Size;
use xilem::widget::align::SingleAlignment;
use xilem::widget::AlignCx;
use xilem::{button, v_stack, Adapt, App, AppLauncher, Memoize, View};
use xilem::{
    widget::{
        self, text::TextWidget, EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod, RawEvent,
        UpdateCx,
    },
    Cx, EventResult, Id, Widget,
};

pub fn both_as_widget(cx: &mut Cx) -> impl Widget {
    let reactive_counter = Pod::new(ViewAsWidget::new(
        cx,
        ReactiveCounter::default(),
        reactive_counter,
    ));
    let sep = Pod::new(TextWidget::new("--------------------------".to_string()));
    let counter = Pod::new(CounterWidget::new(cx));
    let stack = widget::vstack::VStack::new(vec![reactive_counter, sep, counter]);
    stack
}

fn app(data: &mut ()) -> impl View<()> {
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
    pub fn new(cx: &mut Cx) -> Self {
        let widgets = cx
            .with_new_id(|cx| {
                vec![
                    Pod::new(TextWidget::new(format!("Count: {}", 0))),
                    Pod::new(widget::button::Button::new(
                        cx.id_path(),
                        "Increase".to_string(),
                    )),
                ]
            })
            .1;
        CounterWidget {
            count: 0,
            stack: widget::vstack::VStack::new(widgets),
        }
    }
}

impl Widget for CounterWidget {
    fn message(&mut self, _id_path: &[Id], _event: Box<dyn Any>) -> EventResult<()> {
        self.count += 1;

        let label = &mut self.stack.children_mut()[0];
        label.request_update();
        let label: &mut TextWidget = label.downcast_mut().unwrap();
        label.set_text(format!("Count: {}", self.count));
        EventResult::RequestRebuild
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.stack.event(cx, event);
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
#[derive(Default)]
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
}

impl<S, V: View<S>> ViewAsWidget<S, V>
where
    V::Element: Widget + 'static,
{
    fn new(cx: &mut Cx, mut logic_state: S, view_tree_builder: fn(&mut S) -> V) -> Self {
        let view = view_tree_builder(&mut logic_state);
        let (id, view_state, element) = view.build(cx);
        let element = Pod::new(element);
        ViewAsWidget {
            view_tree_builder,
            view,
            view_state,
            logic_state,
            id,
            element,
        }
    }
}

impl<S, V: View<S>> Widget for ViewAsWidget<S, V>
where
    V::Element: Widget + 'static,
{
    fn message(&mut self, id_path: &[Id], event: Box<dyn Any>) -> EventResult<()> {
        let result = self.view.event(
            id_path,
            &mut self.view_state,
            self.element.downcast_mut().unwrap(),
            event,
            &mut self.logic_state,
        );
        let view = (self.view_tree_builder)(&mut self.logic_state);
        let cx = Cx::new();
        let changed = self.view.rebuild(
            &mut cx,
            &self.view,
            &mut self.id,
            &mut self.view_state,
            self.element.downcast_mut().unwrap(),
        );
        if changed {
            self.element.update(cx);
        }
        result
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        //self.root_pod.update(cx);
        //cx.request_layout();
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.element.event(cx, event);
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
struct WidgetAsView<T>(fn(&mut Cx) -> T);

impl<T: Widget> View<(), ()> for WidgetAsView<T> {
    type State = bool;

    type Element = T;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|cx| self.0(cx));
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
        id_path: &[Id],
        state: &mut Self::State,
        element: &mut Self::Element,
        event: Box<dyn Any>,
        _app_state: &mut (),
    ) -> EventResult<()> {
        let result = element.message(id_path, event);
        if let EventResult::RequestRebuild = result {
            *state = true;
        }
        result
    }
}
