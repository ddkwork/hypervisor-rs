//! Handles CPU-related virtualization tasks, specifically intercepting and managing
//! the `CPUID` instruction in a VM to control the exposure of CPU features to the guest.

#![allow(dead_code)]

use {
    crate::intel::{vmexit::VmExit, vmlaunch::GeneralPurposeRegisters},
    bitfield::BitMut,
    x86::cpuid::cpuid,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Enum representing the various CPUID leaves for feature and interface discovery.
/// Reference: https://learn.microsoft.com/en-us/virtualization/hyper-v-on-windows/tlfs/feature-discovery
enum CpuidLeaf {
    /// CPUID function number to retrieve the processor's vendor identification string.
    VendorInfo = 0x0,

    /// CPUID function for feature information, including hypervisor presence.
    FeatureInformation = 0x1,

    /// CPUID function for extended feature information.
    ExtendedFeatureInformation = 0x7,

    /// Hypervisor vendor information leaf.
    HypervisorVendor = 0x40000000,

    /// Hypervisor interface identification leaf.
    HypervisorInterface = 0x40000001,

    /// Hypervisor system identity information leaf.
    HypervisorSystemIdentity = 0x40000002,

    /// Hypervisor feature identification leaf.
    HypervisorFeatureIdentification = 0x40000003,

    /// Hypervisor implementation recommendations leaf.
    ImplementationRecommendations = 0x40000004,

    /// Hypervisor implementation limits leaf.
    HypervisorImplementationLimits = 0x40000005,

    /// Hardware-specific features in use by the hypervisor leaf.
    ImplementationHardwareFeatures = 0x40000006,

    /// Nested hypervisor feature identification leaf.
    NestedHypervisorFeatureIdentification = 0x40000009,

    /// Nested virtualization features available leaf.
    HypervisorNestedVirtualizationFeatures = 0x4000000A,
}

/// Enumerates specific feature bits in the ECX register for CPUID instruction results.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FeatureBits {
    /// Bit 5 of ECX for CPUID with EAX=1, indicating VMX support.
    HypervisorVmxSupportBit = 5,
    /// Bit 31 of ECX for CPUID with EAX=1, indicating hypervisor presence.
    HypervisorPresentBit = 31,
}

/// Handles the `CPUID` VM-exit.
///
/// This function is invoked when the guest executes the `CPUID` instruction.
/// The handler retrieves the results of the `CPUID` instruction executed on
/// the host and then modifies or masks certain bits, if necessary, before
/// returning the results to the guest.
///
/// # Arguments
///
/// * `registers` - A mutable reference to the guest's current register state.
///
/// Reference: Intel® 64 and IA-32 Architectures Software Developer's Manual, Table C-1. Basic Exit Reasons 10.
#[rustfmt::skip]
pub fn handle_cpuid(registers: &mut GeneralPurposeRegisters) {
    let leaf = registers.rax as u32;
    let sub_leaf = registers.rcx as u32;

    // Execute CPUID instruction on the host and retrieve the result
    let mut cpuid_result = cpuid!(leaf, sub_leaf);

    log::info!("Before modification: CPUID Leaf: {:#x}, EAX: {:#x}, EBX: {:#x}, ECX: {:#x}, EDX: {:#x}", leaf, cpuid_result.eax, cpuid_result.ebx, cpuid_result.ecx, cpuid_result.edx);

    match leaf {
        // Handle CPUID for standard feature information.
        leaf if leaf == CpuidLeaf::FeatureInformation as u32 => {
            log::info!("CPUID leaf 1 detected (Standard Feature Information).");
            // Indicate hypervisor presence by setting the appropriate bit in ECX.
            cpuid_result.ecx.set_bit(FeatureBits::HypervisorPresentBit as usize, true);
        },
        // Handle CPUID for hypervisor vendor information.
        leaf if leaf == CpuidLeaf::HypervisorVendor as u32 => {
            log::info!("CPUID leaf 0x40000000 detected (Hypervisor Vendor Information).");
            // Set the CPUID response to provide the hypervisor's vendor ID signature.
            // We use the signature "MatrixVisor" encoded in a little-endian format.
            cpuid_result.eax = CpuidLeaf::HypervisorInterface as u32; // Maximum supported CPUID leaf range.
            cpuid_result.ebx = 0x69727461; // "atri", part of "MatrixVisor" (in reverse order due to little-endian storage).
            cpuid_result.ecx = 0x73695678; // "xVis", part of "MatrixVisor" (in reverse order due to little-endian storage).
            cpuid_result.edx = 0x0000726f; // "or", the final part of "MatrixVisor" followed by two null bytes (in reverse order).
        },
        // Handle CPUID for hypervisor interface identification.
        leaf if leaf == CpuidLeaf::HypervisorInterface as u32 => {
            log::info!("CPUID leaf 0x40000001 detected (Hypervisor Interface Identification).");
            // Return information indicating the hypervisor's interface.
            // Here, we specify that our hypervisor does not conform to the Microsoft hypervisor interface ("Hv#1").
            cpuid_result.eax = 0x00000001; // Interface signature indicating non-conformance to Microsoft interface.
            cpuid_result.ebx = 0x00000000; // Reserved field set to zero.
            cpuid_result.ecx = 0x00000000; // Reserved field set to zero.
            cpuid_result.edx = 0x00000000; // Reserved field set to zero.
        },
        _ => { /* Pass through other CPUID leaves unchanged. */ }
    }

    log::info!("After modification: CPUID Leaf: {:#x}, EAX: {:#x}, EBX: {:#x}, ECX: {:#x}, EDX: {:#x}", leaf, cpuid_result.eax, cpuid_result.ebx, cpuid_result.ecx, cpuid_result.edx);

    // Update the guest registers
    registers.rax = cpuid_result.eax as u64;
    registers.rbx = cpuid_result.ebx as u64;
    registers.rcx = cpuid_result.ecx as u64;
    registers.rdx = cpuid_result.edx as u64;

    VmExit::advance_guest_rip();
}
