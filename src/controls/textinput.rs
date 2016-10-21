/*!
    A control where the user can enter text
*/
/*
    Copyright (C) 2016  Gabriel Dubé

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use std::hash::Hash;

use controls::ControlTemplate;
use controls::base::{WindowBase, create_base, set_window_text, get_window_text,
 get_window_pos, set_window_pos, get_window_size, set_window_size, get_window_parent,
 set_window_parent, get_window_enabled, set_window_enabled, get_window_visibility,
 set_window_visibility, to_utf16, get_control_type};
use actions::{Action, ActionReturn};
use events::Event;
use constants::{HTextAlign, ControlType};

use winapi::{HWND, ES_LEFT, ES_RIGHT, ES_CENTER, WS_BORDER, ES_AUTOHSCROLL, ES_NOHIDESEL,
 ES_PASSWORD, ES_READONLY, EM_SETCUEBANNER, EM_GETCUEBANNER};

/**
    Configuration properties to create a simple TextInput

    * text: The button text
    * size: The button size (width, height) in pixels
    * position: The button position (x, y) in the parent control
    * parent: The control parent
*/
pub struct TextInput<ID: Eq+Clone+Hash> {
    pub text: String,
    pub size: (u32, u32),
    pub position: (i32, i32),
    pub parent: ID,
    pub placeholder: Option<String>,
    pub text_align: HTextAlign,
    pub password: bool,
    pub readonly: bool
}

impl<ID: Eq+Clone+Hash > ControlTemplate<ID> for TextInput<ID> {

    fn create(&self, ui: &mut ::Ui<ID>, id: ID) -> Result<HWND, ()> {
        let h_align = match self.text_align {
            HTextAlign::Left => ES_LEFT,
            HTextAlign::Right => ES_RIGHT,
            HTextAlign::Center => ES_CENTER
        };

        let mut extra = 0;
        if self.password {
            extra |= ES_PASSWORD;
        }
        if self.readonly {
            extra |= ES_READONLY;
        }

        let base = WindowBase::<ID> {
            text: self.text.clone(),
            size: self.size.clone(),
            position: self.position.clone(),
            visible: true,
            resizable: false,
            extra_style: extra | h_align | WS_BORDER | ES_AUTOHSCROLL | ES_NOHIDESEL,
            class: "EDIT".to_string(),
            parent: Some(self.parent.clone())
        };

        let handle = unsafe { create_base::<ID>(ui, base) };
        match handle {
            Ok(h) => {
                 if let Some(placeholder) = self.placeholder.as_ref() {
                     set_placeholder::<ID>(h, Some(Box::new(placeholder.clone())) );
                 }
                 Ok(h)
            }
            e => e
        }
    }

    fn supported_events(&self) -> Vec<Event> {
        vec![Event::MouseUp, Event::MouseDown, Event::Focus, Event::ValueChanged, Event::MaxValue,
             Event::Removed, Event::Resize,]
    }

    fn evaluator(&self) -> ::ActionEvaluator<ID> {
        Box::new( |ui, id, handle, action| {
            match action {
                Action::SetText(t) => set_window_text(handle, *t),
                Action::GetText => get_window_text(handle),
                Action::GetPosition => get_window_pos(handle, true),
                Action::SetPosition(x, y) => set_window_pos(handle, x, y),
                Action::GetSize => get_window_size(handle),
                Action::SetSize(w, h) => set_window_size(handle, w, h),
                Action::GetParent => get_window_parent(handle),
                Action::SetParent(p) => set_window_parent(ui, handle, p, true),
                Action::GetEnabled => get_window_enabled(handle),
                Action::SetEnabled(e) => set_window_enabled(handle, e),
                Action::GetVisibility => get_window_visibility(handle),
                Action::SetVisibility(v) => set_window_visibility(handle, v),
                Action::Reset => set_window_text(handle, "".to_string()),
                Action::GetControlType => get_control_type(handle),

                Action::GetTextLimit => get_text_limit(handle),
                Action::SetTextLimit(l) => set_text_limit(handle, l),
                Action::GetSelectedBounds => get_select_bounds(handle),
                Action::SetSelectedBounds(b) => set_select_bounds(handle, b),
                Action::GetReadonly => get_readonly(handle),
                Action::SetReadonly(r) => set_readonly(handle, r),
                Action::Undo => undo_text(handle),
                Action::GetPlaceholder => get_placeholder(handle),
                Action::SetPlaceholder(p) => set_placeholder(handle, p),

                _ => ActionReturn::NotSupported
            }
        })
    }

    fn control_type(&self) -> ControlType {
        ControlType::TextInput
    }


}

use winapi::{EM_LIMITTEXT, EM_GETLIMITTEXT, UINT, WPARAM, WM_UNDO, EM_GETSEL, DWORD, EM_SETSEL,
 LPARAM, EM_SETREADONLY, GWL_STYLE, LONG_PTR};
use user32::GetWindowLongPtrW;
use controls::base::{send_message};
use std::mem;

fn get_text_limit<ID: Eq+Clone+Hash>(handle: HWND) -> ActionReturn<ID> {
    let limit = send_message(handle, EM_GETLIMITTEXT as UINT, 0, 0) as u32;
    ActionReturn::TextLimit(limit)
}

fn set_text_limit<ID: Eq+Clone+Hash>(handle: HWND, limit: u32) -> ActionReturn<ID> {
    send_message(handle, EM_LIMITTEXT as UINT, limit as WPARAM, 0);
    ActionReturn::None
}

fn undo_text<ID: Eq+Clone+Hash>(handle: HWND) -> ActionReturn<ID> {
    send_message(handle, WM_UNDO as UINT, 0, 0);
    ActionReturn::None
}

fn get_select_bounds<ID: Eq+Clone+Hash>(handle: HWND) -> ActionReturn<ID> {
    let mut min: DWORD = 0;
    let mut max: DWORD = 0;
    
    unsafe{ send_message(handle, EM_GETSEL as u32, mem::transmute(&mut min), mem::transmute(&mut max)) };

    ActionReturn::SelectBounds((min as u32, max as u32))
}

fn set_select_bounds<ID: Eq+Clone+Hash>(handle: HWND, bounds: (u32, u32)) -> ActionReturn<ID> {
    send_message(handle, EM_SETSEL as u32, bounds.0 as WPARAM, bounds.1 as LPARAM);
    ActionReturn::None
}

fn get_readonly<ID: Eq+Clone+Hash>(handle: HWND) -> ActionReturn<ID> { unsafe{
    let read_only = ES_READONLY as LONG_PTR;
    ActionReturn::Readonly( GetWindowLongPtrW(handle, GWL_STYLE) & read_only == read_only )
}}

fn set_readonly<ID: Eq+Clone+Hash>(handle: HWND, readonly: bool) -> ActionReturn<ID> {
    send_message(handle, EM_SETREADONLY as u32, readonly as WPARAM, 0);
    ActionReturn::None
}

fn set_placeholder<ID: Eq+Clone+Hash>(handle: HWND, placeholder: Option<Box<String>> ) -> ActionReturn<ID> {
    let ptr: LPARAM;
    if let Some(placeholder) = placeholder {
        let placeholder_raw = to_utf16(*placeholder);
        ptr = unsafe{ mem::transmute(placeholder_raw.as_ptr()) };
        send_message(handle, EM_SETCUEBANNER, 0, ptr);
    } else {
        let null_string: [u16; 1] = [0];
        ptr = unsafe{ mem::transmute(null_string.as_ptr()) };
        send_message(handle, EM_SETCUEBANNER, 0, ptr);
    }
    ActionReturn::None
}

fn get_placeholder<ID: Eq+Clone+Hash>(handle: HWND) -> ActionReturn<ID> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    // There are no way to get the placeholder length, so the length must be guessed.
    // 256 characters should be enough.
    let mut buffer: [u16; 256] = [0; 256];
    let ptr: WPARAM = unsafe{ mem::transmute(buffer.as_mut_ptr()) };

    send_message(handle, EM_GETCUEBANNER, ptr, 256);

    let end_index = buffer.iter().enumerate().find(|&(index, i)| *i == 0).unwrap_or((256, &0)).0;
    if end_index > 1 {
        let text = OsString::from_wide(&(buffer[0..end_index]));
        let text = text.into_string().unwrap_or("ERROR!".to_string());
        ActionReturn::Text(Box::new(text))
    } else {
        ActionReturn::None
    }

}
