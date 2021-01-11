//This file is completely added for ss and gencopy
// It defines TripleSpaceCopyContext and TripleSpaceProcessEdges
use super::global::TripleSpace;
use crate::plan::CopyContext;
use crate::policy::space::Space;
use crate::scheduler::gc_works::*;
use crate::util::alloc::{Allocator, BumpAllocator};
use crate::util::forwarding_word;
use crate::util::{Address, ObjectReference, OpaquePointer};
use crate::vm::VMBinding;
use crate::MMTK;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub struct TripleSpaceCopyContext<VM: VMBinding> {
    plan:&'static TripleSpace<VM>,
    triplespace: BumpAllocator<VM>,
}

//Filling out copycontext functions relevant for semispace
impl<VM: VMBinding> CopyContext for TripleSpaceCopyContext<VM> {
    type VM = VM;
    fn new(mmtk: &'static MMTK<Self::VM>) -> Self {
        //ALLOC
        Self {
            plan: &mmtk.plan,
            triplespace: BumpAllocator::new(OpaquePointer::UNINITIALIZED, None, &mmtk.plan),
        }
    }
    fn init(&mut self, tls:OpaquePointer) {
        self.triplespace.tls = tls;
    }
    fn prepare(&mut self) {
        //now, the allocator will reallocate live objects to the tospace
        self.triplespace.rebind(Some(self.plan.tospace()));
    }
    fn release(&mut self) {
        // Why is this commented out?
        // self.triplespace.rebind(Some(self.plan.tospace()));
    }
    #[inline(always)]
    fn alloc_copy(
        &mut self,
        _original: ObjectReference,
        bytes: usize,
        align: usize,
        offset: isize,
        _semantics: crate::AllocationSemantics,
    ) -> Address {
        self.triplespace.alloc(bytes, align, offset) //allocates using the bump pointer
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
pub struct TripleSpaceProcessEdges<VM: VMBinding> {
    base: ProcessEdgesBase<TripleSpaceProcessEdges<VM>>,
    phantom: PhantomData<VM>,
}


impl<VM:VMBinding> ProcessEdgesWork for TripleSpaceProcessEdges<VM> {
    type VM = VM;
    fn new(edges: Vec<Address>, _roots: bool) -> Self {
        Self {
            base: ProcessEdgesBase::new(edges),
            ..Default::default()
        }
    }
    // if the object is non-null, checks if it is reachable, copies it if appropriate,
    // and then returns the object.
    // If it is null, just return the object.
    // Implementing stuff from src/scheduler/gc_works.rs
    #[inline]
    fn trace_object(&mut self, object: ObjectReference) -> ObjectReference {
        if object.is_null() {
            //return the object if it is null
            return object;
        }

        else if self.plan().youngspace().in_space(object) {
            self.plan().youngspace.trace_object(
                self, //trace
                object, //object
                super::global::ALLOC_TripleSpace, //semantics
                self.worker().local(), //copycontext
            )
        }

        else if self.plan().tospace().in_space(object) {
            self.plan().tospace().trace_object(
                self,
                object,
                super::global::ALLOC_TripleSpace,
                self.worker().local(),
            )
        } else if self.plan().fromspace().in_space(object) {
            self.plan().fromspace().trace_object(
                self,
                object,
                super::global::ALLOC_TripleSpace,
                self.worker().local(),
            )
        } else {
            self.plan().common.trace_object(self, object)
        }
    }
}

// Returns a reference to the data within TripleSpaceProcessEdges
impl<VM: VMBinding> Deref for TripleSpaceProcessEdges<VM> {
    type Target = ProcessEdgesBase<Self>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<VM: VMBinding> DerefMut for TripleSpaceProcessEdges<VM> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
