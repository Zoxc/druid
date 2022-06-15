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

use druid_shell::kurbo::{Point, Size};

use super::{
    align::{Center, SingleAlignment},
    contexts::LifeCycleCx,
    EventCx, LayoutCx, LifeCycle, PaintCx, Pod, RawEvent, UpdateCx, Widget,
};
use crate::{Cx, EventResult, Id};

pub struct VStack {
    id: Option<Id>,
    children: Vec<Pod>,
    ids: Vec<Option<Id>>,
    alignment: SingleAlignment,
    spacing: f64,
}

impl VStack {
    pub fn new(children: Vec<Pod>) -> Self {
        let alignment = SingleAlignment::from_horiz(&Center);
        let spacing = 0.0;
        VStack {
            ids: children.iter().map(|pod| pod.widget.id()).collect(),
            children,
            alignment,
            spacing,
            id: None,
        }
    }

    pub fn with_id(cx: &mut Cx, children: impl FnOnce(&mut Cx) -> Vec<Pod>) -> Self {
        let (id, mut stack) = cx.with_new_id(|cx| Self::new(children(cx)));
        stack.id = Some(id);
        stack
    }

    pub fn children_mut(&mut self) -> &mut Vec<Pod> {
        &mut self.children
    }
}

impl Widget for VStack {
    fn id(&self) -> Option<Id> {
        self.id
    }

    fn message(&mut self, id_path: &[crate::Id], event: Box<dyn std::any::Any>) -> EventResult<()> {
        let hd = id_path[0];
        let tl = &id_path[1..];
        let child = self
            .ids
            .iter()
            .enumerate()
            .find(|(_, id)| **id == Some(hd))
            .unwrap()
            .0;

        let result = self.children[child].widget.message(tl, event);

        if let EventResult::RequestRebuild = result {
            self.children[child].request_update();
        }

        result
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        for child in &mut self.children {
            child.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        for child in &mut self.children {
            child.lifecycle(cx, event);
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        for child in &mut self.children {
            child.update(cx);
        }
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        let mut min_size = Size::ZERO;
        let mut max_size = Size::ZERO;
        for child in &mut self.children {
            let (child_min, child_max) = child.prelayout(cx);
            min_size.width = min_size.width.max(child_min.width);
            min_size.height += child_min.height;
            max_size.width = max_size.width.max(child_max.width);
            max_size.height += child_max.height;
        }
        let spacing = self.spacing * (self.children.len() - 1) as f64;
        min_size.height += spacing;
        max_size.height += spacing;
        (min_size, max_size)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        // First, sort children in order of increasing flexibility
        let mut child_order: Vec<_> = (0..self.children.len()).collect();
        child_order.sort_by_key(|ix| self.children[*ix].height_flexibility().to_bits());
        // Offer remaining height to each child
        let mut n_remaining = self.children.len();
        let mut height_remaining = proposed_size.height - (n_remaining - 1) as f64 * self.spacing;
        for ix in child_order {
            let child_height = (height_remaining / n_remaining as f64).max(0.0);
            let child_proposed = Size::new(proposed_size.width, child_height);
            let child_size = self.children[ix].layout(cx, child_proposed);
            height_remaining -= child_size.height;
            n_remaining -= 1;
        }
        // Get alignments from children
        let alignments: Vec<f64> = self
            .children
            .iter()
            .map(|child| child.get_alignment(self.alignment))
            .collect();
        let max_align = alignments
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or_default();
        // Place children, using computed height and alignments
        let mut size = Size::default();
        let mut y = 0.0;
        for (i, (child, align)) in self.children.iter_mut().zip(alignments).enumerate() {
            if i != 0 {
                y += self.spacing;
            }
            let child_size = child.state.size;
            let origin = Point::new(max_align - align, y);
            child.state.origin = origin;
            size.width = size.width.max(child_size.width + origin.x);
            y += child_size.height;
        }
        size.height = y;
        size
    }

    fn align(&self, cx: &mut super::AlignCx, alignment: SingleAlignment) {
        for child in &self.children {
            child.align(cx, alignment);
        }
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}
