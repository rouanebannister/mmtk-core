use crate::plan::barriers::NoBarrier;
use crate::plan::mutator_context::Mutator;
use crate::plan::mutator_context::MutatorConfig;
use crate::plan::singlesemi::SingleSemi;
use crate::plan::AllocationSemantics as AllocationType;
use crate::util::alloc::allocators::{AllocatorSelector, Allocators};
use crate::util::OpaquePointer;
use crate::vm::VMBinding;
use enum_map::enum_map;
use enum_map::EnumMap;

lazy_static! {
    pub static ref ALLOCATOR_MAPPING: EnumMap<AllocationType, AllocatorSelector> = enum_map! {
        AllocationType::Default | AllocationType::Immortal | AllocationType::Code | AllocationType::ReadOnly | AllocationType::Los => AllocatorSelector::BumpPointer(0),
    };
}

pub fn singlesemi_mutator_noop<VM: VMBinding>(_mutator: &mut Mutator<SingleSemi<VM>>, _tls: OpaquePointer) {
    unreachable!();
}

pub fn create_singlesemi_mutator<VM: VMBinding>(
    mutator_tls: OpaquePointer,
    plan: &'static SingleSemi<VM>,
) -> Mutator<SingleSemi<VM>> {
    let config = MutatorConfig {
        allocator_mapping: &*ALLOCATOR_MAPPING,
        space_mapping: box vec![(AllocatorSelector::BumpPointer(0), &plan.singlesemi_space)],
        prepare_func: &singlesemi_mutator_noop,
        release_func: &singlesemi_mutator_noop,
    };

    Mutator {
        allocators: Allocators::<VM>::new(mutator_tls, plan, &config.space_mapping),
        barrier: box NoBarrier,
        mutator_tls,
        config,
        plan,
    }
}
