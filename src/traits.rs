//! # Traits for peripherals
//!
//! These traits should ideally come from the embedded-hal, but things like PWM
//! channels and timers aren't specified yet. To still be able to hand them around
//! comfortably into other functions and structs without adding all the generic
//! types you need, these traits come in handy.

use crate::timer::Channel;

pub trait PwmTimer {
    type Error;
    type ChannelIndex;
    type PeriodValue;
    type CompareValue;

    fn enable(&mut self, channel: Channel);
    fn disable(&mut self, channel: Channel);

    fn get_duty(&self, channel: Channel) -> Self::CompareValue;
    fn set_duty(&mut self, channel: Channel, duty: Self::CompareValue);

    fn get_period(&self) -> Self::PeriodValue;
    fn set_period(&mut self, period: Self::PeriodValue) -> Result<(), Self::Error>;

    fn get_max_duty(&self) -> u32;
    fn disable_counter(&mut self);
    fn enable_counter(&mut self);
    fn reset_count(&mut self);
}
