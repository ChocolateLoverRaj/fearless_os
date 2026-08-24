use common::pat::OUR_PAT;
use x86_64::registers::model_specific::Pat;
///
/// # Safety
/// Changes PAT MSR.
pub unsafe fn init() {
    unsafe { Pat::write(OUR_PAT) };
}
