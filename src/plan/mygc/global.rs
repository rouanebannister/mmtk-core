// nDC = noGC didn't collect
use super::gc_works::{MyGCCopyContext, MyGCProcessEdges}; //add. Reason: contexts for copying
use crate::mmtk::MMTK;
use crate::plan::global::BasePlan; //change. Reason: used to include NoCopy, not needed here
use crate::plan::global::CommonPlan; //add. Reason: uses common plan functions
use crate::plan::global::GcStatus; //add. Reason: to keep track of whether or not garbage is being collected. nDC
use crate::plan::mutator_context::Mutator;
use crate::plan::mygc::mutator::create_mygc_mutator;
use crate::plan::mygc::mutator::ALLOCATOR_MAPPING;
use crate::plan::AllocationSemantics;
use crate::plan::Plan;
use crate::policy::copyspace::CopySpace; //add. Reason: uses copyspaces
use crate::policy::space::Space;
use crate::scheduler::gc_works::*; //add. Reason: To schedule collections. nDC
use crate::scheduler::*; //change. Reason: See above.
use crate::util::alloc::allocators::AllocatorSelector;
use crate::util::heap::layout::heap_layout::Mmapper;
use crate::util::heap::layout::heap_layout::VMMap;
use crate::util::heap::layout::vm_layout_constants::{HEAP_END, HEAP_START};
use crate::util::heap::HeapMeta;
use crate::util::heap::VMRequest;
use crate::util::options::UnsafeOptionsWrapper;
#[cfg(feature = "sanity")] //add. Reason: This and below allow import of sanity GC checker
use crate::util::sanity::sanity_checker::*; //add. see abv
use crate::util::OpaquePointer;
use crate::vm::VMBinding;
use std::sync::atomic::{AtomicBool, Ordering}; //add. Reason: SS needs to order its memory storage. 
use std::sync::Arc;
use enum_map::EnumMap;
//remove ln15 (allow unused imports). Reason: No unused imports
// remove ln23-6 lock free lines. Reason: Not relevant

pub type SelectedPlan<VM> = MyGC<VM>;

pub const ALLOC_MyGC: AllocationSemantics = AllocationSemantics::Default; //add. Reason: ?

pub struct MyGC<VM: VMBinding> {
    //change - whole thing here is changed
    // Reason: It's a different GC needing different things. NoGC doesn't need a bool or multiple spaces.
    pub hi: AtomicBool, // indicating which space is to/from
    pub copyspace0: CopySpace<VM>,
    pub copyspace1: CopySpace<VM>,
    // 2x copyspaces. Tospace, fromspace, currently not specified
    pub common: CommonPlan<VM>, 
}

unsafe impl<VM: VMBinding> Sync for MyGC<VM> {}

impl<VM: VMBinding> Plan for MyGC<VM> {
    type VM = VM;
    type Mutator = Mutator<Self>;
    type CopyContext = MyGCCopyContext<VM>; //change. Reason: Old was NoCopy

    fn new(
        vm_map: &'static VMMap,
        mmapper: &'static Mmapper,
        options: Arc<UnsafeOptionsWrapper>,
        _scheduler: &'static MMTkScheduler<Self::VM>,
    ) -> Self {
        //change - again, completely changed.
        let mut heap = HeapMeta::new(HEAP_START, HEAP_END);

        MyGC {
            hi: AtomicBool::new(false),
            copyspace0: CopySpace::new(
                "copyspace0",
                false,
                true,
                VMRequest::discontiguous(),
                vm_map,
                mmapper,
                &mut heap,
            ),
            copyspace1: CopySpace::new(
                "copyspace1",
                true,
                true,
                VMRequest::discontiguous(),
                vm_map,
                mmapper,
                &mut heap,
            ),
            common: CommonPlan::new(vm_map, mmapper, options, heap),
        }
    }

    fn gc_init(
        &mut self, 
        heap_size: usize,
        vm_map: &'static VMMap,
        scheduler: &Arc<MMTkScheduler<VM>>,
    ) {
        //change, Reason: to use common and init copyspaces
        self.common.gc_init(heap_size, vm_map, scheduler);
        self.copyspace0.init(&vm_map);
        self.copyspace1.init(&vm_map);
    }

    //change. Reason: calls unreachable in nogc
    fn schedule_collection(&'static self, scheduler:&MMTkScheduler<VM>) {
        self.base().set_collection_kind();
        self.base().set_gc_status(GcStatus::GcPrepare);
        //stop. scan mutators
        scheduler.unconstrained_works
            .add(StopMutators::<MyGCProcessEdges<VM>>::new());
        // prep global/coll/mut
        scheduler.prepare_stage.add(Prepare::new(self));
        // release global/coll/mut
        scheduler.release_stage.add(Release::new(self));
        // resume mutators
        #[cfg(feature = "sanity")]
        scheduler.final_stage.add(ScheduleSanityGC);
        scheduler.set_finalizer(Some(EndOfGC));
    }

    fn bind_mutator(
        &'static self,
        tls: OpaquePointer,
        _mmtk: &'static MMTK<Self::VM>,
    ) -> Box<Mutator<Self>> {
        Box::new(create_mygc_mutator(tls, self))
    }

    fn get_allocator_mapping(&self) -> &'static EnumMap<AllocationSemantics, AllocatorSelector> {
        &*ALLOCATOR_MAPPING
    }

    //prepares the spaces
    fn prepare(&self, tls: OpaquePointer) {
        //add. Reason: NoGC just calls unreachable
        self.common.prepare(tls, true);

        self.hi
            .store(!self.hi.load(Ordering::SeqCst), Ordering::SeqCst);
        // Flips 'hi' to flip space definitions
        let hi = self.hi.load(Ordering::SeqCst); 
        self.copyspace0.prepare(hi); // Prep spaces w new definition
                                     // of which one is to and from
        self.copyspace1.prepare(!hi);
    }

    fn release(&self, tls: OpaquePointer) {
        //add. Reason: NoGC just calls unreachable.
        //releases the thread storage, and anything
        //left in fromspace
        self.common.release(tls, true);
        self.fromspace().release();
    }

    //these ones are just making referencing easier.
    //add. Reason: ?
    fn get_collection_reserve(&self) -> usize {
        self.tospace().reserved_pages()
    }
 
    //change. Reason: refer to correct space
    fn get_pages_used(&self) -> usize {
        self.tospace().reserved_pages() + self.common.get_pages_used()
    }

    //change. Reason: Common instead of base plan
    fn base(&self) -> &BasePlan<VM> {
        &self.common.base
    }

    //add. Reason: Makes referencing CommonPlan functions easier.
    fn common(&self) -> &CommonPlan<VM> {
        &self.common
    }
}

//alloc
//add. Reason: adds references to to/from spaces, not used in nogc
impl<VM: VMBinding> MyGC<VM> {
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

//remove handle_user_collection_request. Reason: Handled in global instance, nogc thing is an override.