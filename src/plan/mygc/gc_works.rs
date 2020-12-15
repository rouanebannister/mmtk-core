//This file is completely added for ss and gencopy
// It defines MyGCCopyContext and MyGCProcessEdges
use super::global::MyGC;
use crate::plan::CopyContext;
use crate::policy::space::Space;
use crate::scheduler::gc_works::*;
use crate::util::alloc::{Allocator, BumpAllocator}; //ALLOC
use crate::util::forwarding_word;
use crate::util::{Address, ObjectReference, OpaquePointer};
use crate::vm::VMBinding;
use crate::MMTK;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub struct MyGCCopyContext<VM: VMBinding> {
    plan:&'static MyGC<VM>,
    mygc: BumpAllocator<VM>,
}

//Filling out copycontext functions relevant for semispace
impl<VM: VMBinding> CopyContext for MyGCCopyContext<VM> {
    type VM = VM;
    fn new(mmtk: &'static MMTK<Self::VM>) -> Self {
        //ALLOC
        Self {
            plan: &mmtk.plan,
            mygc: BumpAllocator::new(OpaquePointer::UNINITIALIZED, None, &mmtk.plan),
        }
    }
    fn init(&mut self, tls:OpaquePointer) {
        self.mygc.tls = tls;
    }
    fn prepare(&mut self) {
        self.mygc.rebind(Some(self.plan.tospace()));
    }
    fn release(&mut self) {
        // Why is this commented out?
        // self.mygc.rebind(Some(self.plan.tospace()));
    }
    #[inline(always)]
    fn alloc_copy(
        //ALLOC
        &mut self,
        _original: ObjectReference,
        bytes: usize,
        align: usize,
        offset: isize,
        _semantics: crate::AllocationSemantics,
    ) -> Address {
        self.mygc.alloc(bytes, align, offset)
    }
    #[inline(always)]
    fn post_copy(
        &mut self,
        obj: ObjectReference,
        _tib: Address,
        _bytes: usize,
        _semantics: crate::AllocationSemantics,
    ) {
        forwarding_word::clear_forwarding_bits::<VM>(obj);
    }
}

#[derive(Default)]
pub struct MyGCProcessEdges<VM: VMBinding> {
    base: ProcessEdgesBase<MyGCProcessEdges<VM>>,
    phantom: PhantomData<VM>,
}


impl<VM:VMBinding> ProcessEdgesWork for MyGCProcessEdges<VM> {
    type VM = VM;
    fn new(edges: Vec<Address>, _roots: bool) -> Self {
        Self {
            base: ProcessEdgesBase::new(edges),
            ..Default::default()
        }
    }
    // if the object is non-null, logs its reference to the trace
    // and regardless, returns the reference to the object.
    // Implementing stuff from src/scheduler/gc_works.rs
    #[inline]
    fn trace_object(&mut self, object: ObjectReference) -> ObjectReference {
        if object.is_null() {
            //return the object if it is null
            return object;
        }
        if self.plan().tospace().in_space(object) {
            // if the object is in the tospace, use tospace's trace
            // and return the result
            self.plan().tospace().trace_object(
                self,
                object,
                super::global::ALLOC_MyGC,
                self.worker().local(),
            )
        } else if self.plan().fromspace().in_space(object) {
            // if the object is in the fromspace, use fromspace's trace
            // and return the result
            self.plan().fromspace().trace_object(
                self,
                object,
                super::global::ALLOC_MyGC,
                self.worker().local(),
            )
        } else {
            // if it's in neither space, use common's trace
            // and return the result
            self.plan().common.trace_object(self, object)
        }
    }
}

// Returns a reference to the data within MyGCProcessEdges
impl<VM: VMBinding> Deref for MyGCProcessEdges<VM> {
    type Target = ProcessEdgesBase<Self>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<VM: VMBinding> DerefMut for MyGCProcessEdges<VM> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
