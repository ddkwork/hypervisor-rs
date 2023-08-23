extern crate alloc;
use alloc::boxed::Box;

use crate::{
    error::HypervisorError,
    intel::{vmexit::launch_vm, vmx::Vmx},
    utils::context::Context,
};

pub struct Vcpu {
    /// The index of the processor.
    index: u32,

    /// Whether the processor is virtualized or not.
    is_virtualized: bool,
}

impl Vcpu {
    pub fn new(index: u32) -> Result<Self, HypervisorError> {
        log::debug!("Creating processor {}", index);

        Ok(Self {
            index,
            is_virtualized: false,
        })
    }

    /// Virtualize the CPU by capturing the context, enabling VMX operation, adjusting control registers, calling VMXON, VMPTRLD and VMLAUNCH
    pub fn virtualize_cpu(&mut self, context: Context) -> Result<(), HypervisorError> {
        log::info!("[+] Virtualizing processor {}", self.index);

        if !self.is_virtualized {
            self.is_virtualized = true;

            log::info!("[+] Initializing VMX");
            let v = Vmx::new(context)?;
            let mut vmx = Box::new(v);

            vmx.init()?;

            log::info!("[+] Running the guest until VM-exit occurs.");

            // Run the VM until the VM-exit occurs.
            unsafe { launch_vm() };
        }

        Ok(())
    }

    /// Gets the index of the current logical/virtual processor
    pub fn id(&self) -> u32 {
        self.index
    }

    pub fn is_virtualized(&self) -> bool {
        self.is_virtualized
    }
}