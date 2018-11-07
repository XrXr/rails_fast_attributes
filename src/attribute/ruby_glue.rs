use std::cell::Cell;

use ffi;
use into_ruby::*;
use super::{Attribute, MaybeProc, Source};
use util::*;

impl IntoRuby for Attribute {
    unsafe fn class() -> ffi::VALUE {
        ATTRIBUTE.unwrap()
    }

    unsafe fn mark(&self) {
        use self::Source::*;

        match *self {
            Attribute::Populated {
                name,
                ref raw_value,
                ty,
                ref source,
                ref value,
            } => {
                ffi::rb_gc_mark(name);
                raw_value.mark();
                ffi::rb_gc_mark(ty);
                match *source {
                    FromUser(ref orig) => orig.mark(),
                    UserProvidedDefault(Some(ref orig)) => orig.mark(),
                    UserProvidedDefault(None) | FromDatabase | PreCast => {} // noop
                }
                if let Some(value) = value.get() {
                    ffi::rb_gc_mark(value);
                }
            }
            Attribute::Uninitialized { name, ty } => {
                ffi::rb_gc_mark(name);
                ffi::rb_gc_mark(ty);
            }
        }
    }
}

impl MaybeProc {
    unsafe fn mark(&self) {
        use self::MaybeProc::*;

        match *self {
            NotProc(value) => ffi::rb_gc_mark(value),
            Proc { block, ref memo } => {
                ffi::rb_gc_mark(block);
                if let Some(memo) = memo.get() {
                    ffi::rb_gc_mark(memo);
                }
            }
        }
    }
}

static mut ATTRIBUTE: Option<ffi::VALUE> = None;

pub unsafe fn init() {
    let attribute = ffi::rb_define_class_under(::module(), cstr!("Attribute"), ffi::rb_cObject);
    ATTRIBUTE = Some(attribute);

    ffi::rb_define_alloc_func(attribute, Attribute::allocate);
    ffi::rb_define_singleton_method(
        attribute,
        cstr!("from_database"),
        from_database as *const _,
        3,
    );
    ffi::rb_define_singleton_method(attribute, cstr!("from_user"), from_user as *const _, 4);
    ffi::rb_define_singleton_method(
        attribute,
        cstr!("with_cast_value"),
        from_cast_value as *const _,
        3,
    );
    ffi::rb_define_singleton_method(
        attribute,
        cstr!("uninitialized"),
        uninitialized as *const _,
        2,
    );
    ffi::rb_define_singleton_method(
        attribute,
        cstr!("user_provided_default"),
        user_provided_default as *const _,
        4,
    );

    ffi::rb_define_method(
        attribute,
        cstr!("value_before_type_cast"),
        value_before_type_cast as *const _,
        0,
    );
    ffi::rb_define_method(attribute, cstr!("name"), name as *const _, 0);
    ffi::rb_define_method(attribute, cstr!("type"), ty as *const _, 0);
    ffi::rb_define_method(attribute, cstr!("value"), value as *const _, 0);
    ffi::rb_define_method(
        attribute,
        cstr!("original_value"),
        original_value as *const _,
        0,
    );
    ffi::rb_define_method(
        attribute,
        cstr!("value_for_database"),
        value_for_database as *const _,
        0,
    );
    ffi::rb_define_method(attribute, cstr!("changed?"), changed_eh as *const _, 0);
    ffi::rb_define_method(
        attribute,
        cstr!("changed_in_place?"),
        changed_in_place_eh as *const _,
        0,
    );
    ffi::rb_define_method(
        attribute,
        cstr!("forgetting_assignment"),
        forgetting_assignment as *const _,
        0,
    );
    ffi::rb_define_method(
        attribute,
        cstr!("with_value_from_user"),
        with_value_from_user as *const _,
        1,
    );
    ffi::rb_define_method(
        attribute,
        cstr!("with_value_from_database"),
        with_value_from_database as *const _,
        1,
    );
    ffi::rb_define_method(
        attribute,
        cstr!("with_cast_value"),
        with_cast_value as *const _,
        1,
    );
    ffi::rb_define_method(attribute, cstr!("with_type"), with_type as *const _, 1);
    ffi::rb_define_method(
        attribute,
        cstr!("initialized?"),
        initialized_eh as *const _,
        0,
    );
    ffi::rb_define_method(
        attribute,
        cstr!("came_from_user?"),
        came_from_user as *const _,
        0,
    );
    ffi::rb_define_method(
        attribute,
        cstr!("has_been_read?"),
        has_been_read as *const _,
        0,
    );
    ffi::rb_define_method(attribute, cstr!("=="), equals as *const _, 1);
    ffi::rb_define_method(attribute, cstr!("eql?"), equals as *const _, 1);
    ffi::rb_define_method(attribute, cstr!("hash"), hash as *const _, 0);
    ffi::rb_define_method(
        attribute,
        cstr!("initialize_dup"),
        initialize_dup as *const _,
        1,
    );
    ffi::rb_define_method(attribute, cstr!("marshal_dump"), dump_data as *const _, 0);
    ffi::rb_define_method(attribute, cstr!("marshal_load"), load_data as *const _, 1);
    ffi::rb_define_method(attribute, cstr!("encode_with"), encode_with as *const _, 1);
    ffi::rb_define_method(
        attribute,
        cstr!("init_with"),
        init_with as *const _,
        1,
    );
}

fn from_value(value: ffi::VALUE) -> Attribute {
    unsafe {
        if ffi::rb_obj_class(value) == Attribute::class() {
            get_struct::<Attribute>(value).clone()
        } else {
            ffi::rb_raise(ffi::rb_eRuntimeError, cstr!("Expected an `Attribute`"))
        }
    }
}

extern "C" fn from_database(
    _class: ffi::VALUE,
    name: ffi::VALUE,
    value: ffi::VALUE,
    ty: ffi::VALUE,
) -> ffi::VALUE {
    Attribute::from_database(name, value, ty).into_ruby()
}

extern "C" fn from_user(
    _class: ffi::VALUE,
    name: ffi::VALUE,
    value: ffi::VALUE,
    ty: ffi::VALUE,
    original_attribute: ffi::VALUE,
) -> ffi::VALUE {
    let original_attribute = from_value(original_attribute);
    Attribute::from_user(name, value, ty, original_attribute).into_ruby()
}

extern "C" fn from_cast_value(
    _class: ffi::VALUE,
    name: ffi::VALUE,
    value: ffi::VALUE,
    ty: ffi::VALUE,
) -> ffi::VALUE {
    Attribute::from_cast_value(name, value, ty).into_ruby()
}

extern "C" fn uninitialized(_class: ffi::VALUE, name: ffi::VALUE, ty: ffi::VALUE) -> ffi::VALUE {
    Attribute::uninitialized(name, ty).into_ruby()
}

extern "C" fn user_provided_default(
    _class: ffi::VALUE,
    name: ffi::VALUE,
    value: ffi::VALUE,
    ty: ffi::VALUE,
    original_attribute: ffi::VALUE,
) -> ffi::VALUE {
    let proc_c = unsafe { ffi::rb_const_get(ffi::rb_cObject, id!("Proc")) };
    let value = unsafe {
        if ffi::RTEST(ffi::rb_funcall(value, id!("is_a?"), 1, proc_c)) {
            MaybeProc::Proc {
                block: value,
                memo: Default::default(),
            }
        } else {
            MaybeProc::NotProc(value)
        }
    };

    let original_attribute = unsafe {
        if ffi::RB_NIL_P(original_attribute) {
            None
        } else {
            Some(get_struct::<Attribute>(original_attribute).clone())
        }
    };
    Attribute::user_provided_default(name, value, ty, original_attribute).into_ruby()
}

extern "C" fn value_before_type_cast(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.value_before_type_cast()
}

extern "C" fn name(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.name()
}

extern "C" fn ty(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.ty()
}

extern "C" fn value(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.value()
}

extern "C" fn original_value(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.original_value()
}

extern "C" fn value_for_database(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.value_for_database()
}

extern "C" fn changed_eh(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    to_ruby_bool(this.is_changed())
}

extern "C" fn changed_in_place_eh(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    to_ruby_bool(this.is_changed_in_place())
}

extern "C" fn forgetting_assignment(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.forgetting_assignment().into_ruby()
}

extern "C" fn with_value_from_user(this: ffi::VALUE, value: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.clone().with_value_from_user(value).into_ruby()
}

extern "C" fn with_value_from_database(this: ffi::VALUE, value: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.with_value_from_database(value).into_ruby()
}

extern "C" fn with_cast_value(this: ffi::VALUE, value: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.with_cast_value(value).into_ruby()
}

extern "C" fn with_type(this: ffi::VALUE, ty: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    this.with_type(ty).into_ruby()
}

extern "C" fn initialized_eh(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    to_ruby_bool(this.is_initialized())
}

extern "C" fn came_from_user(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    to_ruby_bool(this.came_from_user())
}

extern "C" fn has_been_read(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<Attribute>(this) };
    to_ruby_bool(this.has_been_read())
}

extern "C" fn equals(this: ffi::VALUE, other: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        if !ffi::RB_TYPE_P(other, ffi::T_DATA) {
            return ffi::Qfalse;
        }
        if ffi::rb_obj_class(other) != Attribute::class() {
            return ffi::Qfalse;
        }

        let this = get_struct::<Attribute>(this);
        let other = get_struct::<Attribute>(other);
        to_ruby_bool(this == other)
    }
}

extern "C" fn hash(this: ffi::VALUE) -> ffi::VALUE {
    use self::Attribute::*;
    use self::Source::*;

    unsafe {
        let this = get_struct::<Attribute>(this);
        let discriminant = match *this {
            Uninitialized { .. } => 0,
            Populated {
                source: FromUser(_),
                ..
            } => 1,
            Populated {
                source: FromDatabase,
                ..
            } => 2,
            Populated {
                source: PreCast, ..
            } => 3,
            Populated {
                source: UserProvidedDefault(_),
                ..
            } => 4,
        };
        let discriminant = ffi::I322NUM(discriminant);
        let name = this.name();
        let value = this.value_before_type_cast();
        let ty = this.ty();

        let ary = to_ruby_array(4, vec![discriminant, name, value, ty]);
        ffi::rb_funcall(ary, id!("hash"), 0)
    }
}

extern "C" fn initialize_dup(this: ffi::VALUE, other: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct_mut::<Attribute>(this) };
    let other = unsafe { get_struct::<Attribute>(other) };
    this.initialize_dup(other);
    unsafe { ffi::Qnil }
}

extern "C" fn dump_data(this: ffi::VALUE) -> ffi::VALUE {
    use self::Attribute::*;
    let this = unsafe { get_struct::<Attribute>(this) };

    return match *this {
        Populated {
            name,
            ref raw_value,
            ty,
            ref source,
            value: ref _value,
        } => to_ruby_array(4, vec![name, ty, raw_value.value(), dump_source(source)]),
        Uninitialized { name, ty } => to_ruby_array(2, vec![name, ty]),
    };
}

extern "C" fn load_data(this: ffi::VALUE, data: ffi::VALUE) -> ffi::VALUE {
    use self::Attribute::*;
    use self::MaybeProc::*;

    unsafe {
        let this = get_struct_mut::<Attribute>(this);
        let name = ffi::rb_ary_entry(data, 0);
        let ty = ffi::rb_ary_entry(data, 1);
        let raw_value = NotProc(ffi::rb_ary_entry(data, 2));
        let source = ffi::rb_ary_entry(data, 3);

        if ffi::RB_NIL_P(source) {
            *this = Uninitialized { name, ty };
        } else {
            let source = load_source(source);
            *this = Populated {
                name,
                ty,
                raw_value,
                source,
                value: Cell::new(None),
            };
        }

        ffi::Qnil
    }
}

extern "C" fn encode_with(this: ffi::VALUE, coder: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let this = get_struct::<Attribute>(this);
        let value_before_type_cast = this.value_before_type_cast();
        let ty = this.ty();

        ffi::rb_funcall(coder, id!("[]="), 2, rstr!("name"), this.name());

        if !ffi::RB_NIL_P(ty) {
            ffi::rb_funcall(coder, id!("[]="), 2, rstr!("type"), ty);
        }

        if !ffi::RB_NIL_P(value_before_type_cast) {
            ffi::rb_funcall(
                coder,
                id!("[]="),
                2,
                rstr!("value_before_type_cast"),
                value_before_type_cast,
            );
        }

        if let Some(orig) = this.original_attribute() {
            ffi::rb_funcall(
                coder,
                id!("[]="),
                2,
                rstr!("original_attribute"),
                orig.clone().into_ruby(),
            );
        }

        if this.has_been_read() {
            ffi::rb_funcall(coder, id!("[]="), 2, rstr!("value"), this.value());
        }

        lie_about_our_class(this, coder);

        ffi::Qnil
    }
}

fn lie_about_our_class(this: &Attribute, coder: ffi::VALUE) {
    use self::Attribute::*;
    use self::Source::*;

    let class_name = match *this {
        Populated {
            source: FromUser(..),
            ..
        } => "FromUser",
        Populated {
            source: FromDatabase,
            ..
        } => "FromDatabase",
        Populated {
            source: PreCast, ..
        } => "WithCastValue",
        Populated {
            source: UserProvidedDefault(..),
            ..
        } => "UserProvidedDefault",
        Uninitialized { .. } => "Uninitialized",
    };
    let tag = format!("!ruby/object:ActiveModel::Attribute::{}", class_name);
    // This method is definitely not meant to override the tag,
    // but it's the only method in the public API that lets us do it without
    // other side effects
    unsafe {
        ffi::rb_funcall(coder, id!("map"), 1, rstr!(&tag));
    }
}

/// Contains the common logic of `init_with` for all populated variants.
/// Source will be set to `Source::PreCast`.
unsafe fn init_with_populated(this: &mut Attribute, coder: ffi::VALUE) {
    let name = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("name"));
    let ty = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("type"));
    let raw_value = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("value_before_type_cast"));
    let value = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("value"));

    let value = if ffi::RB_NIL_P(value) {
        None
    } else {
        Some(value)
    };

    *this = Attribute::Populated {
        name,
        ty,
        raw_value: MaybeProc::NotProc(raw_value),
        source: Source::PreCast,
        value: Cell::new(value),
    };
}

extern "C" fn init_with_from_database(this: ffi::VALUE, coder: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let this = get_struct_mut::<Attribute>(this);
        init_with_populated(this, coder);
        match *this {
            Attribute::Populated { ref mut source, .. } => *source = Source::FromDatabase,
            _ => unreachable!(),
        }
        ffi::Qnil
    }
}

extern "C" fn init_with_from_user(this: ffi::VALUE, coder: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let this = get_struct_mut::<Attribute>(this);
        init_with_populated(this, coder);
        let original_attribute = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("original_attribute"));
        let new_source = if ffi::RB_NIL_P(original_attribute) {
            // Even though this was a `FromUser` subclass, if this YAML was
            // dumped in Rails 4.2, `original_attribute` won't be there.
            // `UserProvidedDefault` is the only thing that can have no original.
            Source::UserProvidedDefault(None)
        } else {
            Source::FromUser(Box::new(
                get_struct::<Attribute>(original_attribute).clone()
            ))
        };

        match *this {
            Attribute::Populated { ref mut source, .. } => {
                *source = new_source
            },
            _ => unreachable!(),
        }

        ffi::Qnil
    }
}

extern "C" fn init_with(this: ffi::VALUE, coder: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        use std::slice;
        use std::str;

        let tag = ffi::rb_funcall(coder, id!("tag"), 0);
        let tag_ptr = ffi::RSTRING_PTR(tag);
        let tag_len = ffi::RSTRING_LEN(tag);
        let tag = slice::from_raw_parts(tag_ptr as _, tag_len as usize);

        let tag_prefix_len = b"!ruby/object:ActiveModel::Attribute::".len();

        match &tag[tag_prefix_len..] {
            b"FromUser" | b"UserProvidedDefault" => init_with_from_user(this, coder),
            b"FromDatabase" => init_with_from_database(this, coder),
            b"WithCastValue" => init_with_precast(this, coder),
            b"Uninitialized" => init_with_uninitialized(this, coder),
            other => {
                let tag_str = str::from_utf8(other).unwrap_or("{invalid UTF-8}");
                // We don't get the whole tag if there is a 0 byete in the middle of the tag.
                let error = format!("Unrecognized tag: {}\0", tag_str);
                ffi::rb_raise(ffi::rb_eTypeError, cstr!("%s"), error.as_bytes().as_ptr());
            }
        }
    }
}

extern "C" fn init_with_precast(this: ffi::VALUE, coder: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let this = get_struct_mut::<Attribute>(this);
        init_with_populated(this, coder);

        ffi::Qnil
    }
}

extern "C" fn init_with_uninitialized(this: ffi::VALUE, coder: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let this = get_struct_mut::<Attribute>(this);
        let name = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("name"));
        let ty = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("type"));

        *this = Attribute::Uninitialized { name, ty };

        ffi::Qnil
    }
}

fn dump_source(source: &'static Source) -> ffi::VALUE {
    use self::Source::*;
    let discriminant = match *source {
        FromUser(_) => 1,
        FromDatabase => 2,
        PreCast => 3,
        UserProvidedDefault(_) => 4,
    };
    let original_attr = match *source {
        FromUser(ref orig) | UserProvidedDefault(Some(ref orig)) => orig.clone().into_ruby(),
        _ => unsafe { ffi::Qnil },
    };
    let discriminant = unsafe { ffi::I322NUM(discriminant) };
    to_ruby_array(2, vec![discriminant, original_attr])
}

fn load_source(source: ffi::VALUE) -> Source {
    use self::Source::*;

    fn error() -> ! {
        unsafe { ffi::rb_raise(ffi::rb_eRuntimeError, cstr!("Unrecognized attribute")) };
    }

    unsafe {
        let discriminant = ffi::rb_ary_entry(source, 0);
        let attr = ffi::rb_ary_entry(source, 1);
        let attr = if ffi::RB_NIL_P(attr) {
            None
        } else {
            Some(Box::new(get_struct::<Attribute>(attr).clone()))
        };
        match ffi::NUM2I32(discriminant) {
            1 => FromUser(attr.unwrap()),
            2 => FromDatabase,
            3 => PreCast,
            4 => UserProvidedDefault(attr),
            _ => error(),
        }
    }
}
