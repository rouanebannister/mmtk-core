use crate::mmtk::MMTK;
use crate::plan::global::{BasePlan, NoCopy}; //change. Reason: used to include NoCopy, not needed here
use crate::plan::global::CommonPlan; //add. Reason: uses common plan functions
use crate::plan::mutator_context::Mutator;
use crate::plan::withmut::mutator::create_withmut_mutator;
use crate::plan::withmut::mutator::ALLOCATOR_MAPPING;
use crate::plan::AllocationSemantics;
use crate::plan::Plan;
use crate::policy::copyspace::CopySpace; //add. Reason: uses copyspaces
use crate::policy::space::Space;
use crate::scheduler::*; //change. Reason: See above.
use crate::util::alloc::allocators::AllocatorSelector;
use crate::util::heap::layout::heap_layout::Mmapper;
use crate::util::heap::layout::heap_layout::VMMap;
use crate::util::heap::layout::vm_layout_constants::{HEAP_END, HEAP_START};
use crate::util::heap::HeapMeta;
use crate::util::heap::VMRequest;
use crate::util::options::UnsafeOptionsWrapper;
use crate::util::OpaquePointer;
use crate::vm::VMBinding;
use std::sync::atomic::{AtomicBool, Ordering}; //add. Reason: SS needs to order its memory storage. 
use std::sync::Arc;
use enum_map::EnumMap;

pub type SelectedPlan<VM> = WithMut<VM>;

pub struct WithMut<VM: VMBinding> {
    pub hi: AtomicBool,
    pub copyspace0: CopySpace<VM>,
    pub copyspace1: CopySpace<VM>,
    pub common: CommonPlan<VM>,
}

unsafe impl<VM: VMBinding> Sync for WithMut<VM> {}

impl<VM: VMBinding> Plan for WithMut<VM> {
    type VM = VM;
    type Mutator = Mutator<Self>;
    type CopyContext = NoCopy<VM>; 

    fn new(
        vm_map: &'static VMMap,
        mmapper: &'static Mmapper,
        options: Arc<UnsafeOptionsWrapper>,
        _scheduler: &'static MMTkScheduler<Self::VM>,
    ) -> Self {
        let mut heap = HeapMeta::new(HEAP_START, HEAP_END);

        let copyspace0 = CopySpace::new(
            "copyspace0",
            false,
            true,
            VMRequest::discontiguous(),
            vm_map,
            mmapper,
            &mut heap,
         );
         let copyspace1 = CopySpace::new(
            "copyspace1",
            true,
            true,
            VMRequest::discontiguous(),
            vm_map,
            mmapper,
            &mut heap,
         );
        WithMut {
            hi: AtomicBool::new(false),
            copyspace0,
            copyspace1,
            common: CommonPlan::new(vm_map, mmapper, options, heap),
        }
    }

    fn gc_init(
        &mut self,
        heap_size: usize,
        vm_map: &'static VMMap,
        scheduler: &Arc<MMTkScheduler<VM>>,
    ) {
        self.common.gc_init(heap_size, vm_map, scheduler);
        self.copyspace0.init(&vm_map);
        self.copyspace1.init(&vm_map);
    }

    fn base(&self) -> &BasePlan<VM> {
        &self.common.base
    }

    fn common(&self) -> &CommonPlan<VM> {
        &self.common
      }

    fn bind_mutator(
        &'static self,
        tls: OpaquePointer,
        _mmtk: &'static MMTK<Self::VM>,
    ) -> Box<Mutator<Self>> {
        Box::new(create_withmut_mutator(tls, self))
    }

    fn prepare(&self, _tls: OpaquePointer) {
        unreachable!()
    }

    fn release(&self, _tls: OpaquePointer) {
        unreachable!()
    }

    fn get_allocator_mapping(&self) -> &'static EnumMap<AllocationSemantics, AllocatorSelector> {
        &*ALLOCATOR_MAPPING
    }

    fn schedule_collection(&'static self, _scheduler: &MMTkScheduler<VM>) {
        unreachable!("GC triggered in withmut")
    }

    fn get_pages_used(&self) -> usize {
        self.tospace().reserved_pages() + self.common.get_pages_used()
    }

    fn handle_user_collection_request(&self, _tls: OpaquePointer, _force: bool) {
        println!("Warning: User attempted a collection request, but it is not supported in WithMut. The request is ignored.");
    }
}

impl<VM: VMBinding> WithMut<VM> {
    pub fn tospace(&self) -> &CopySpace<VM> {
        // if hi then tospace == 1
        // else tospace == 0
        if self.hi.load(Ordering::SeqCst) {
            &self.copyspace1
        } else {
            &self.copyspace0
        }
    }

    pub fn fromspace(&self) -> &CopySpace<VM> {
        // gets reverse of self.tospace().
        if self.hi.load(Ordering::SeqCst) {
            &self.copyspace0
        } else {
            &self.copyspace1
        }
    }
}
