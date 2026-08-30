// SPDX-License-Identifier: MPL-2.0

use crate::math::IVec2;
use smithay::{desktop::Window, utils::IsAlive};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct Grid {
    pub cells: HashMap<IVec2, Window>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    pub fn find_nearest_empty(&self, start: IVec2) -> IVec2 {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(curr) = queue.pop_front() {
            if !self.cells.contains_key(&curr) {
                return curr;
            }

            for dir_flipped in IVec2::AXES {
                let dir = dir_flipped * IVec2::FLIP_Y;
                let next = curr + dir;
                if !visited.contains(&next) {
                    visited.insert(next);
                    queue.push_back(next);
                }
            }
        }
        unreachable!();
    }

    pub fn insert(&mut self, pos: IVec2, window: Window) {
        self.cells.insert(pos, window);
    }

    pub fn get(&mut self, pos: &IVec2) -> Option<Window> {
        self.cells.get(pos).cloned()
    }

    pub fn cleanup(&mut self) {
        self.cells.retain(|_, window| window.alive());
    }

    pub fn swap(&mut self, src: IVec2, dst: IVec2) {
        // if src != dst {
        //     if let Some([a, b]) = self.cells.get_many_mut([&src, &dst]) {
        //         std::mem::swap(a, b);
        //     }

        if src == dst {
            return;
        }

        let src_window = self.cells.remove(&src);
        let dst_window = self.cells.remove(&dst);

        if let Some(src_window) = src_window {
            self.cells.insert(dst, src_window);
        }

        if let Some(dst_window) = dst_window {
            self.cells.insert(src, dst_window);
        }
    }
}
