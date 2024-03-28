use std::{collections::HashMap, time};

use floem::reactive::RwSignal;
use ndoc::Document;

#[derive(Debug, Clone)]
pub struct Documents {
    documents: HashMap<usize,(time::Instant, RwSignal<Document>)>,
    current: usize,
}

impl Documents {
    pub fn new() -> Self {
        Documents {
            documents: HashMap::new(),
            current: 0,
        }
    }
    pub fn add(&mut self, doc: RwSignal<Document>) {
        self.documents.insert(doc.get().id(), (time::Instant::now(), doc));
        self.current = doc.get().id();
    }
    pub fn remove(&mut self, id: usize) {
        self.documents.remove(&id);
        if !self.documents.is_empty() {
            self.current = *self.documents.iter().max_by(|v1,v2| v1.0.cmp(v2.0)).unwrap().0;
        } else {
            self.current = 0;
        }
    }
    pub fn current(&self) -> RwSignal<Document> {
        self.documents[&self.current].1
    }
    pub fn current_id(&self) -> usize {
        self.current
    }
    pub fn set_current(&mut self, id: usize) {
        self.current = id;
        self.documents.get_mut(&id).unwrap().0 = time::Instant::now();
    }
    pub fn get_doc(&self, id: usize) -> RwSignal<Document> {
        self.documents[&id].1
    }
    pub fn order_by_mru(&self) -> im::Vector<RwSignal<Document>> {
        let mut v = self.documents.values().clone().collect::<im::Vector<_>>();
        v.sort_by(|l,r| r.0.cmp(&l.0));
        im::Vector::from_iter(v.iter().map(|(_,d)| *d))
    }
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
    pub fn len(&self) -> usize {
        self.documents.len()
    }
}