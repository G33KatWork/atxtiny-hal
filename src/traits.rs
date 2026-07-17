//! # Traits for peripherals
//!
//! These traits should ideally come from the embedded-hal, but things like PWM
//! channels and timers aren't specified yet. To still be able to hand them around
//! comfortably into other functions and structs without adding all the generic
//! types you need, these traits come in handy.

/// A PWM-capable timer with runtime-indexed channels.
///
/// Duty cycles live in a unified `u32` domain: `0` is constant-low and
/// [`get_max_duty`](Self::get_max_duty) (the period in ticks, i.e. the PER
/// register value plus one) is constant-high — a compare value above TOP
/// never matches, so the output never switches off. Values larger than the
/// compare register can hold are clamped to the register maximum; that only
/// matters when the period spans the full register range, where true
/// constant-high output is unachievable by one tick.
pub trait PwmTimer {
    type Error;
    type ChannelIndex;
    type PeriodValue;

    fn enable(&mut self, channel: Self::ChannelIndex) -> Result<(), Self::Error>;
    fn disable(&mut self, channel: Self::ChannelIndex) -> Result<(), Self::Error>;

    fn get_duty(&self, channel: Self::ChannelIndex) -> Result<u32, Self::Error>;
    fn set_duty(&mut self, channel: Self::ChannelIndex, duty: u32) -> Result<(), Self::Error>;

    fn get_period(&self) -> Self::PeriodValue;
    fn set_period(&mut self, period: Self::PeriodValue) -> Result<(), Self::Error>;

    /// The duty value that produces a constant-high output (100%).
    fn get_max_duty(&self) -> u32;

    fn disable_counter(&mut self);
    fn enable_counter(&mut self);
    fn reset_count(&mut self);
}
