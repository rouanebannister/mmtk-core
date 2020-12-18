use crate::mmtk::MMTK;
use crate::plan::global::{BasePlan, NoCopy};
use crate::plan::mutator_context::Mutator;
use crate::plan::tutetest::mutator::create_tutetest_mutator;
use crate::plan::tutetest::mutator::ALLOCATOR_MAPPING;
use crate::plan::AllocationSemantics;
use crate::plan::Plan;
use crate::policy::space::Space;
use crate::scheduler::MMTkScheduler;
use crate::util::alloc::allocators::AllocatorSelector;
use crate::util::heap::layout::heap_layout::Mmapper;
use crate::util::heap::layout::heap_layout::VMMap;
use crate::util::heap::layout::vm_layout_constants::{HEAP_END, HEAP_START};
use crate::util::heap::HeapMeta;
#[allow(unused_imports)]
use crate::util::heap::VMRequest;
use crate::util::options::UnsafeOptionsWrapper;
use crate::util::OpaquePointer;
use crate::vm::VMBinding;
use enum_map::EnumMap;
use std::sync::Arc;

use crate::policy::immortalspace::ImmortalSpace as TuteTestImmortalSpace;

pub type SelectedPlan<VM> = TuteTest<VM>;

pub struct TuteTest<VM: VMBinding> {
    pub base: BasePlan<VM>,
    pub tutetest_space: TuteTestImmortalSpace<VM>,
}

unsafe impl<VM: VMBinding> Sync for TuteTest<VM> {}

impl<VM: VMBinding> Plan for TuteTest<VM> {
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

        let tutetest_space = TuteTestImmortalSpace::new(
            "tutetest_space",
            true,
            VMRequest::discontiguous(),
            vm_map,
            mmapper,
            &mut heap,
        );

        TuteTest {
            tutetest_space,
            base: BasePlan::new(vm_map, mmapper, options, heap),
        }
    }

    fn gc_init(
        &mut self,
        heap_size: usize,
        vm_map: &'static VMMap,
        scheduler: &Arc<MMTkScheduler<VM>>,
    ) {
        self.base.gc_init(heap_size, vm_map, scheduler);

        // FIXME correctly initialize spaces based on options
        self.tutetest_space.init(&vm_map);
    }

    fn base(&self) -> &BasePlan<VM> {
        &self.base
    }

    fn bind_mutator(
        &'static self,
        tls: OpaquePointer,
        _mmtk: &'static MMTK<Self::VM>,
    ) -> Box<Mutator<Self>> {
        Box::new(create_tutetest_mutator(tls, self))
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
        unreachable!("GC triggered in tutetest")
    }

    fn get_pages_used(&self) -> usize {
        self.tutetest_space.reserved_pages()
    }

    fn handle_user_collection_request(&self, _tls: OpaquePointer, _force: bool) {
        println!("Warning: User attempted a collection request, but it is not supported in TuteTest. The request is ignored.");
    }
}
