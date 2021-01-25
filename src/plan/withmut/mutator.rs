use crate::plan::barriers::NoBarrier;
use crate::plan::mutator_context::Mutator;
use crate::plan::mutator_context::MutatorConfig;
use crate::plan::withmut::WithMut;
use crate::plan::AllocationSemantics as AllocationType;
use crate::util::alloc::allocators::{AllocatorSelector, Allocators};
use crate::util::OpaquePointer;
use crate::vm::VMBinding;
use enum_map::enum_map;
use enum_map::EnumMap;

lazy_static! {
    pub static ref ALLOCATOR_MAPPING: EnumMap<AllocationType, AllocatorSelector> = enum_map! {
        AllocationType::Default  => AllocatorSelector::BumpPointer(0),
        AllocationType::Immortal |
        AllocationType::Code | 
        AllocationType::ReadOnly => AllocatorSelector::BumpPointer(1), 
        AllocationType::Los => AllocatorSelector::LargeObject(0),
    };
}

pub fn withmut_mutator_noop<VM: VMBinding>(_mutator: &mut Mutator<WithMut<VM>>, _tls: OpaquePointer) {
    unreachable!();
}

pub fn create_withmut_mutator<VM: VMBinding>(
    mutator_tls: OpaquePointer,
    plan: &'static WithMut<VM>,
) -> Mutator<WithMut<VM>> {
    let config = MutatorConfig {
        allocator_mapping: &*ALLOCATOR_MAPPING,
        space_mapping: box vec![
            (AllocatorSelector::BumpPointer(0), plan.tospace()),
            (AllocatorSelector::BumpPointer(1), plan.common.get_immortal()),
            (AllocatorSelector::LargeObject(0), plan.common.get_los()),
            ],
        prepare_func: &withmut_mutator_noop,
        release_func: &withmut_mutator_noop,
    };

    Mutator {
        allocators: Allocators::<VM>::new(mutator_tls, plan, &config.space_mapping),
        barrier: box NoBarrier,
        mutator_tls,
        config,
        plan,
    }
}
