#![allow(
    one_public_type_per_file,
    raw_primitive_field,
    raw_primitive_param,
    unsorted_impl_methods,
    dead_code
)]

pub struct Item;
pub struct ItemList;

pub struct Store;

impl Store {
    pub fn list(&self) -> ItemList {
        ItemList
    }
    pub fn put(&self, _item: Item) {}
    pub fn rebuild(&self, _other: Item) -> ItemList {
        let db = self.list();
        db
    }
    pub fn sync(&self, item: Item) {
        self.put(item);
    }
    pub fn uses_private_helper(&self) -> ItemList {
        self.helper()
    }
    fn helper(&self) -> ItemList {
        ItemList
    }
}

pub struct Field {
    pub child: Child,
}

pub struct Child;

impl Child {
    pub fn method(&self) {}
}

impl Field {
    pub fn calls_method_on_field(&self) {
        self.child.method();
    }
}

pub trait DoThing {
    fn list(&self);
}

pub struct WithTrait;

impl WithTrait {
    pub fn list(&self) {}
}

impl DoThing for WithTrait {
    fn list(&self) {
        self.list();
    }
}

fn main() {
    let store = Store;
    let _ = store.rebuild(Item);
    store.sync(Item);
    let _ = store.uses_private_helper();
}
