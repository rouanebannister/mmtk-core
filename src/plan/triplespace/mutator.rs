use super::TripleSpace; //add
use crate::plan::barriers::NoBarrier;
use crate::plan::mutator_context::Mutator;
use crate::plan::mutator_context::MutatorConfig;
use crate::plan::AllocationSemantics as AllocationType;
use crate::util::alloc::allocators::{AllocatorSelector, Allocators};
use crate::util::alloc::BumpAllocator; //add
use crate::util::OpaquePointer;
use crate::vm::VMBinding;
use enum_map::enum_map;
use enum_map::EnumMap;
//remove ln4 crate::plan::nogc::NoGC - same as ln1 here?
//remove nogc_mutator_noop

//add
pub fn triplespace_mutator_prepare<VM: VMBinding>(
    _mutator: &mut Mutator <TripleSpace<VM>>,
    _tls: OpaquePointer,
) { }

//add
pub fn triplespace_mutator_release<VM: VMBinding> (
    mutator: &mut Mutator<TripleSpace<VM>>,
    _tls: OpaquePointer
) {
    let bump_allocator = unsafe {
        mutator
            .allocators
            . get_allocator_mut(
                mutator.config.allocator_mapping[AllocationType::Default]
            )
        }
        .downcast_mut::<BumpAllocator<VM>>()
        .unwrap();
        bump_allocator.rebind(Some(mutator.plan.youngspace()));
}


lazy_static! {
    //change - a lot of the values mapped here are changed
    pub static ref ALLOCATOR_MAPPING: EnumMap<AllocationType, AllocatorSelector> = enum_map! {
        AllocationType::Default => AllocatorSelector::BumpPointer(0),
        AllocationType::Immortal | AllocationType::Code | AllocationType::ReadOnly => AllocatorSelector::BumpPointer(1),
        AllocationType::Los => AllocatorSelector::LargeObject(0),
    };
}

pub fn create_triplespace_mutator<VM: VMBinding>(
    mutator_tls: OpaquePointer,
    plan: &'static TripleSpace<VM>,
) -> Mutator<TripleSpace<VM>> {
    let config = MutatorConfig {
        allocator_mapping: &*ALLOCATOR_MAPPING,
        //maps out memory for the two spaces??
        //change - different mapping bc of multiple spaces
        space_mapping: box vec![
            (AllocatorSelector::BumpPointer(0), plan.youngspace()),
            (
                AllocatorSelector::BumpPointer(1),
                plan.common.get_immortal(),
            ),
            (AllocatorSelector::LargeObject(0), plan.common.get_los()),
        ],
        //change from both being noop   
        prepare_func: &triplespace_mutator_prepare,
        release_func: &triplespace_mutator_release,
    };

    Mutator {
        allocators: Allocators::<VM>::new(mutator_tls, plan, &config.space_mapping),
        barrier: box NoBarrier, //no r/w barrier
        mutator_tls,
        config,
        plan,
    }
}
