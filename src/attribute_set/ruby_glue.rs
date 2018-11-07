use indexmap::IndexMap;

use attribute::Attribute;
use {ffi, libc};
use into_ruby::{IntoRuby};
use super::AttributeSet;
use util::*;

impl IntoRuby for AttributeSet {
    unsafe fn class() -> ffi::VALUE {
        ATTRIBUTE_SET.unwrap()
    }

    unsafe fn mark(&self) {
        for (key, value) in &self.attributes {
            let sym = ffi::rb_id2sym(*key);
            ffi::rb_gc_mark(sym);
            value.mark()
        }
    }

    fn into_ruby(self) -> ffi::VALUE {
        let class = unsafe { ATTRIBUTE_SET.unwrap() };
        allocate_attribute_set(class, self)
    }
}

impl AttributeSet {
    fn lend_to_ruby(&self, attribute_in_map: &'static Attribute) -> ffi::VALUE {
        let attribute_handle = IntoRuby::as_ruby(attribute_in_map);
        unsafe {
            let ruby_self = self.ruby_self.unwrap();
            // TODO(alan) could be a rb_ivar_set but we don't have that binding yet
            ffi::rb_funcall(
                attribute_handle,
                id!("instance_variable_set"),
                2,
                ffi::rb_id2sym(id!("@_parent_attribute_set")),
                ruby_self
            );
        }
        attribute_handle
    }
}

fn allocate_attribute_set(class: ffi::VALUE, set: AttributeSet) -> ffi::VALUE {
    let ptr = Box::into_raw(Box::new(set));
    let ruby_self = unsafe { ffi::Data_Wrap_Struct(class, AttributeSet::mark_ptr, AttributeSet::destroy_ptr, ptr as *mut _) };
    unsafe {
        (*ptr).ruby_self = Some(ruby_self);
    }
    ruby_self
}

extern "C" fn default_allocate(class: ffi::VALUE) -> ffi::VALUE {
    allocate_attribute_set(class, AttributeSet::default())
}

static mut ATTRIBUTE_SET: Option<ffi::VALUE> = None;

pub unsafe fn init() {
    let attribute_set =
        ffi::rb_define_class_under(::module(), cstr!("AttributeSet"), ffi::rb_cObject);
    ATTRIBUTE_SET = Some(attribute_set);

    ffi::rb_define_alloc_func(attribute_set, default_allocate);

    ffi::rb_define_method(
        attribute_set,
        cstr!("initialize"),
        initialize as *const _,
        1,
    );
    ffi::rb_define_method(attribute_set, cstr!("fetch"), fetch as *const _, 1);
    ffi::rb_define_method(
        attribute_set,
        cstr!("each_value"),
        each_value as *const _,
        0,
    );
    ffi::rb_define_method(attribute_set, cstr!("[]"), get as *const _, 1);
    ffi::rb_define_method(attribute_set, cstr!("[]="), set as *const _, 2);
    ffi::rb_define_method(
        attribute_set,
        cstr!("values_before_type_cast"),
        values_before_type_cast as *const _,
        0,
    );
    ffi::rb_define_method(attribute_set, cstr!("to_hash"), to_hash as *const _, 0);
    ffi::rb_define_method(attribute_set, cstr!("to_h"), to_hash as *const _, 0);
    ffi::rb_define_method(attribute_set, cstr!("key?"), key_eh as *const _, 1);
    ffi::rb_define_method(attribute_set, cstr!("keys"), keys as *const _, 0);
    ffi::rb_define_method(
        attribute_set,
        cstr!("fetch_value"),
        fetch_value as *const _,
        1,
    );
    ffi::rb_define_method(
        attribute_set,
        cstr!("write_from_database"),
        write_from_database as *const _,
        2,
    );
    ffi::rb_define_method(
        attribute_set,
        cstr!("write_from_user"),
        write_from_user as *const _,
        2,
    );
    ffi::rb_define_method(
        attribute_set,
        cstr!("write_cast_value"),
        write_cast_value as *const _,
        2,
    );
    ffi::rb_define_method(attribute_set, cstr!("deep_dup"), deep_dup as *const _, 0);
    ffi::rb_define_method(attribute_set, cstr!("reset"), reset as *const _, 1);
    ffi::rb_define_method(
        attribute_set,
        cstr!("initialize_copy"),
        initialize_copy as *const _,
        1,
    );
    ffi::rb_define_method(attribute_set, cstr!("accessed"), accessed as *const _, 0);
    ffi::rb_define_method(attribute_set, cstr!("map"), map as *const _, 0);
    ffi::rb_define_method(attribute_set, cstr!("=="), equals as *const _, 1);
    ffi::rb_define_method(attribute_set, cstr!("marshal_dump"), dump_data as *const _, 0);
    ffi::rb_define_method(attribute_set, cstr!("marshal_load"), load_data as *const _, 1);
    ffi::rb_define_method(attribute_set, cstr!("init_with"), init_with as *const _, 1);
    ffi::rb_define_method(attribute_set, cstr!("except"), except as *const _, -1);
}

extern "C" fn initialize(this: ffi::VALUE, attrs: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let this = get_struct_mut::<AttributeSet>(this);
        this.attributes.reserve(ffi::RHASH_SIZE(attrs) as usize);
        ffi::rb_hash_foreach(
            attrs,
            push_attribute,
            &mut this.attributes as *mut _ as *mut _,
        );

        extern "C" fn push_attribute(
            key: ffi::VALUE,
            value: ffi::VALUE,
            hash_ptr: *mut libc::c_void,
        ) -> ffi::st_retval {
            let hash_ptr = hash_ptr as *mut IndexMap<ffi::ID, Attribute>;
            let hash = unsafe { hash_ptr.as_mut().unwrap() };

            let id = string_or_symbol_to_id(key);
            let value = unsafe { get_struct::<Attribute>(value) }.clone();

            hash.insert(id, value);

            ffi::st_retval::ST_CONTINUE
        }

        ffi::Qnil
    }
}

extern "C" fn fetch(this: ffi::VALUE, name: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    let key = string_or_symbol_to_id(name);
    match this.get(key) {
        Some(attribute) => this.lend_to_ruby(attribute),
        None => unsafe { ffi::rb_yield(ffi::Qnil) }
    }
}

extern "C" fn each_value(this: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        if !ffi::rb_block_given_p() {
            return ffi::rb_funcall(this, id!("to_enum"), 1, ffi::rb_id2sym(id!("each_value")));
        }

        let this = get_struct::<AttributeSet>(this);
        this.each_value(|value| {
            ffi::rb_yield(this.lend_to_ruby(value));
        });
        ffi::Qnil
    }
}

extern "C" fn get(this: ffi::VALUE, name: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    let key = string_or_symbol_to_id(name);
    match this.get(key) {
        Some(attribute) => this.lend_to_ruby(attribute),
        None => unsafe { ffi::rb_funcall(Attribute::class(), id!("null"), 1, name) }
    }
}

extern "C" fn set(this: ffi::VALUE, key: ffi::VALUE, value: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct_mut::<AttributeSet>(this) };
    let attr = unsafe { get_struct::<Attribute>(value) };
    let key = string_or_symbol_to_id(key);
    this.set(key, attr.clone());
    unsafe { ffi::Qnil }
}

extern "C" fn values_before_type_cast(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    this.values_before_type_cast()
}

extern "C" fn to_hash(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    this.to_hash()
}

extern "C" fn key_eh(this: ffi::VALUE, key: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    let key = string_or_symbol_to_id(key);
    to_ruby_bool(this.has_key(key))
}

extern "C" fn keys(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    this.keys()
}

extern "C" fn fetch_value(this: ffi::VALUE, key: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    let key = string_or_symbol_to_id(key);
    this.fetch_value(key).unwrap_or(unsafe { ffi::Qnil })
}

extern "C" fn write_from_database(
    this: ffi::VALUE,
    key: ffi::VALUE,
    value: ffi::VALUE,
) -> ffi::VALUE {
    let this = unsafe { get_struct_mut::<AttributeSet>(this) };
    let key = string_or_symbol_to_id(key);
    this.write_from_database(key, value);
    unsafe { ffi::Qnil }
}

extern "C" fn write_from_user(this: ffi::VALUE, key: ffi::VALUE, value: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct_mut::<AttributeSet>(this) };
    let key = string_or_symbol_to_id(key);
    this.write_from_user(key, value);
    unsafe { ffi::Qnil }
}

extern "C" fn write_cast_value(this: ffi::VALUE, key: ffi::VALUE, value: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct_mut::<AttributeSet>(this) };
    let key = string_or_symbol_to_id(key);
    this.write_cast_value(key, value);
    unsafe { ffi::Qnil }
}

extern "C" fn deep_dup(this_ptr: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this_ptr) };

    this.deep_dup().into_ruby()
}

extern "C" fn reset(this: ffi::VALUE, key: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct_mut::<AttributeSet>(this) };
    if unsafe { !ffi::RB_NIL_P(key) } {
        let key = string_or_symbol_to_id(key);
        this.reset(key);
    }
    unsafe { ffi::Qnil }
}

extern "C" fn initialize_copy(this_ptr: ffi::VALUE, other: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct_mut::<AttributeSet>(this_ptr) };
    let other = unsafe { get_struct::<AttributeSet>(other) };
    this.clone_from(other);
    this_ptr
}

extern "C" fn accessed(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    this.accessed()
}

extern "C" fn map(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    let keep_alive = unsafe { ffi::rb_ary_new_capa(this.attributes.len() as isize) };
    this.map(|attr| unsafe {
        let new_attr = ffi::rb_yield(attr.as_ruby());
        ffi::rb_ary_push(keep_alive, new_attr);
        get_struct::<Attribute>(new_attr).clone()
    }).into_ruby()
}

extern "C" fn equals(this: ffi::VALUE, other: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        if !ffi::RB_TYPE_P(other, ffi::T_DATA) {
            return ffi::Qfalse;
        }
        if ffi::rb_obj_class(other) != AttributeSet::class() {
            return ffi::Qfalse;
        }

        let this = get_struct::<AttributeSet>(this);
        let other = get_struct::<AttributeSet>(other);
        to_ruby_bool(this.attributes == other.attributes)
    }
}

extern "C" fn dump_data(this: ffi::VALUE) -> ffi::VALUE {
    let this = unsafe { get_struct::<AttributeSet>(this) };
    to_ruby_array(
        this.attributes.len(),
        this.attributes.values().map(|attribute| this.lend_to_ruby(attribute)),
    )
}

extern "C" fn load_data(this: ffi::VALUE, data: ffi::VALUE) -> ffi::VALUE {
    use std::slice;
    unsafe {
        let this = get_struct_mut::<AttributeSet>(this);
        let attrs =
            slice::from_raw_parts(ffi::RARRAY_CONST_PTR(data), ffi::RARRAY_LEN(data) as usize);
        this.attributes = attrs
            .iter()
            .map(|value| {
                let attr = get_struct::<Attribute>(*value);
                let key = string_or_symbol_to_id(attr.name());
                (key, attr.clone())
            })
            .collect();
        ffi::Qnil
    }
}

extern "C" fn init_with(this: ffi::VALUE, coder: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let attributes = ffi::rb_funcall(coder, id!("[]"), 1, rstr!("attributes"));
        let hash = ffi::rb_const_get(ffi::rb_cObject, id!("Hash"));
        let attributes = if ffi::RTEST(ffi::rb_funcall(coder, id!("is_a?"), 1, hash)) {
            attributes
        } else {
            ffi::rb_funcall(attributes, id!("materialize"), 0)
        };
        initialize(this, attributes);
        ffi::Qnil
    }
}

extern "C" fn except(argc: libc::c_int, argv: *const ffi::VALUE, this: ffi::VALUE) -> ffi::VALUE {
    unsafe {
        let this = get_struct::<AttributeSet>(this);
        let result = ffi::rb_hash_new();

        for attr in this.attributes.values() {
            ffi::rb_hash_aset(result, attr.name(), this.lend_to_ruby(attr));
        }

        ffi::rb_funcallv(result, id!("except"), argc, argv)
    }
}
