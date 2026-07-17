use super::{Error, FTimer, Instance, Timer, WithPwm};

use fugit::TimerDurationU32;

use crate::time::Hertz;

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

/// The portmux returns a `WaveformOutputPinset` for muxed pins to be
/// used as PWM waveform output pins. What pins can be muxed into a waveform
/// output pin depends on the specific chip.
pub trait WaveformOutputPinset<TCA, const CHAN: u8> {}

/// PWM channel selector.
///
/// C4-C6 only exist on the six-channel split-mode TCA
/// ([`TCASplit`](super::tca_split::TCASplit)); on three-channel timers
/// they are rejected by [`Pins::check_used`] at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Channel {
    C1 = 0,
    C2 = 1,
    C3 = 2,
    C4 = 3,
    C5 = 4,
    C6 = 5,
}

pub struct Ch<const C: u8>;
// Plain consts used as const-generic arguments. While the (enabled)
// min_generic_const_args feature is active, a bare named const in type
// position is rejected — even inside braces — so every use site wraps
// these in a non-trivial anonymous-const expression: `Ch<{ 0 + C1 }>`
// (same workaround as the USERROW_SIZE array length in nvmctrl.rs).
pub const C1: u8 = 0;
pub const C2: u8 = 1;
pub const C3: u8 = 2;
pub const C4: u8 = 3;
pub const C5: u8 = 4;
pub const C6: u8 = 5;

pub trait Pins<TIM, P> {
    const C1: bool = false;
    const C2: bool = false;
    const C3: bool = false;
    const C4: bool = false;
    const C5: bool = false;
    const C6: bool = false;
    type Channels;

    fn check_used(c: Channel) -> Result<Channel, Error> {
        if (c == Channel::C1 && Self::C1)
            || (c == Channel::C2 && Self::C2)
            || (c == Channel::C3 && Self::C3)
            || (c == Channel::C4 && Self::C4)
            || (c == Channel::C5 && Self::C5)
            || (c == Channel::C6 && Self::C6)
        {
            Ok(c)
        } else {
            Err(Error::InvalidChannel)
        }
    }

    fn split() -> Self::Channels;
}

pub struct PwmChannel<TIM, const C: u8> {
    pub(super) _tim: PhantomData<TIM>,
}

pub trait PwmPin<TIM, const C: u8> {}

macro_rules! pins_impl {
    ( $( ( $($PINX:ident),+ ), ( $($ENCHX:ident),+ ); )+ ) => {
        $(
            #[allow(unused_parens)]
            impl<TIM, $($PINX,)+> Pins<TIM, ($(Ch<{ 0 + $ENCHX }>),+)> for ($($PINX),+)
            where
                TIM: Instance + WithPwm,
                $($PINX: PwmPin<TIM, { 0 + $ENCHX }>,)+
            {
                $(const $ENCHX: bool = true;)+
                type Channels = ($(PwmChannel<TIM, { 0 + $ENCHX }>),+);
                fn split() -> Self::Channels {
                    ($(PwmChannel::<TIM, { 0 + $ENCHX }>::new()),+)
                }
            }
        )+
    };
}

// Every non-empty subset of the six channels (C4-C6 only materialize for
// timers whose pinsets provide those channels, i.e. split-mode TCA).
// Multiple impls share a tuple arity; inference still resolves uniquely
// because each pinset type satisfies `PwmPin` for exactly its channel.
pins_impl!(
    (P1, P2, P3, P4, P5, P6), (C1, C2, C3, C4, C5, C6);
    (P1, P2, P3, P4, P5), (C1, C2, C3, C4, C5);
    (P1, P2, P3, P4, P6), (C1, C2, C3, C4, C6);
    (P1, P2, P3, P5, P6), (C1, C2, C3, C5, C6);
    (P1, P2, P4, P5, P6), (C1, C2, C4, C5, C6);
    (P1, P3, P4, P5, P6), (C1, C3, C4, C5, C6);
    (P2, P3, P4, P5, P6), (C2, C3, C4, C5, C6);
    (P1, P2, P3, P4), (C1, C2, C3, C4);
    (P1, P2, P3, P5), (C1, C2, C3, C5);
    (P1, P2, P3, P6), (C1, C2, C3, C6);
    (P1, P2, P4, P5), (C1, C2, C4, C5);
    (P1, P2, P4, P6), (C1, C2, C4, C6);
    (P1, P2, P5, P6), (C1, C2, C5, C6);
    (P1, P3, P4, P5), (C1, C3, C4, C5);
    (P1, P3, P4, P6), (C1, C3, C4, C6);
    (P1, P3, P5, P6), (C1, C3, C5, C6);
    (P1, P4, P5, P6), (C1, C4, C5, C6);
    (P2, P3, P4, P5), (C2, C3, C4, C5);
    (P2, P3, P4, P6), (C2, C3, C4, C6);
    (P2, P3, P5, P6), (C2, C3, C5, C6);
    (P2, P4, P5, P6), (C2, C4, C5, C6);
    (P3, P4, P5, P6), (C3, C4, C5, C6);
    (P1, P2, P3), (C1, C2, C3);
    (P1, P2, P4), (C1, C2, C4);
    (P1, P2, P5), (C1, C2, C5);
    (P1, P2, P6), (C1, C2, C6);
    (P1, P3, P4), (C1, C3, C4);
    (P1, P3, P5), (C1, C3, C5);
    (P1, P3, P6), (C1, C3, C6);
    (P1, P4, P5), (C1, C4, C5);
    (P1, P4, P6), (C1, C4, C6);
    (P1, P5, P6), (C1, C5, C6);
    (P2, P3, P4), (C2, C3, C4);
    (P2, P3, P5), (C2, C3, C5);
    (P2, P3, P6), (C2, C3, C6);
    (P2, P4, P5), (C2, C4, C5);
    (P2, P4, P6), (C2, C4, C6);
    (P2, P5, P6), (C2, C5, C6);
    (P3, P4, P5), (C3, C4, C5);
    (P3, P4, P6), (C3, C4, C6);
    (P3, P5, P6), (C3, C5, C6);
    (P4, P5, P6), (C4, C5, C6);
    (P1, P2), (C1, C2);
    (P1, P3), (C1, C3);
    (P1, P4), (C1, C4);
    (P1, P5), (C1, C5);
    (P1, P6), (C1, C6);
    (P2, P3), (C2, C3);
    (P2, P4), (C2, C4);
    (P2, P5), (C2, C5);
    (P2, P6), (C2, C6);
    (P3, P4), (C3, C4);
    (P3, P5), (C3, C5);
    (P3, P6), (C3, C6);
    (P4, P5), (C4, C5);
    (P4, P6), (C4, C6);
    (P5, P6), (C5, C6);
    (P1), (C1);
    (P2), (C2);
    (P3), (C3);
    (P4), (C4);
    (P5), (C5);
    (P6), (C6);
);

macro_rules! tuples {
    ( $( $trait:ident, ( $($PX:ident),+ ); )+ ) => {
        $(
            impl<TIM, $($PX,)+ const C: u8> $trait<TIM, C> for ($($PX),+)
            where
                $($PX: WaveformOutputPinset<TIM, C>,)+
            {
            }
        )+
    };
}

tuples! {
    WaveformOutputPinset, (P1, P2);
    WaveformOutputPinset, (P1, P2, P3);
}

impl<P, TIM, const C: u8> PwmPin<TIM, C> for P where P: WaveformOutputPinset<TIM, C> {}

pub trait PwmExt<TIM: Instance + WithPwm>
where
    Self: Sized + Instance + WithPwm,
{
    fn pwm<P, PINS, const FREQ: u32>(
        self,
        pins: PINS,
        time: TimerDurationU32<FREQ>,
        mode: Self::GenerationMode,
        clk: TIM::ClockSource,
    ) -> Result<Pwm<Self, P, PINS, FREQ>, Error>
    where
        PINS: Pins<Self, P>;

    fn pwm_hz<P, PINS>(
        self,
        pins: PINS,
        freq: Hertz,
        mode: TIM::GenerationMode,
        clk: TIM::ClockSource,
    ) -> Result<PwmHz<Self, P, PINS>, Error>
    where
        PINS: Pins<Self, P>;
}

impl<TIM: Instance + WithPwm> PwmExt<TIM> for TIM
where
    Self: Sized + Instance + WithPwm,
{
    fn pwm<P, PINS, const FREQ: u32>(
        self,
        pins: PINS,
        time: TimerDurationU32<FREQ>,
        mode: Self::GenerationMode,
        clk: TIM::ClockSource,
    ) -> Result<Pwm<TIM, P, PINS, FREQ>, Error>
    where
        PINS: Pins<Self, P>,
    {
        FTimer::<Self, FREQ>::new(self, clk)?.pwm(pins, time, mode)
    }

    fn pwm_hz<P, PINS>(
        self,
        pins: PINS,
        time: Hertz,
        mode: TIM::GenerationMode,
        clk: TIM::ClockSource,
    ) -> Result<PwmHz<TIM, P, PINS>, Error>
    where
        PINS: Pins<Self, P>,
    {
        Timer::new(self, clk).pwm_hz(pins, time, mode)
    }
}

impl<TIM: Instance + WithPwm, const C: u8> PwmChannel<TIM, C> {
    pub(crate) fn new() -> Self {
        Self {
            _tim: core::marker::PhantomData,
        }
    }
}

// Duty cycles use the unified u32 domain described on
// [`crate::traits::PwmTimer`]: 0 is constant-low, `get_max_duty()`
// (period ticks, PER + 1) is constant-high, larger values clamp to the
// compare register maximum.
impl<TIM: Instance + WithPwm, const C: u8> PwmChannel<TIM, C> {
    #[inline]
    pub fn disable(&mut self) {
        TIM::enable_channel(C, false);
    }

    #[inline]
    pub fn enable(&mut self) {
        TIM::enable_channel(C, true);
    }

    #[inline]
    pub fn get_duty(&self) -> u32 {
        TIM::read_compare_value(C).into()
    }

    #[inline]
    pub fn set_duty(&mut self, duty: u32) {
        TIM::set_compare_value_clamped(C, duty);
    }

    #[inline]
    pub fn get_max_duty(&self) -> u32 {
        TIM::read_period().into() + 1
    }
}

impl<TIM: Instance + WithPwm, const C: u8> crate::embedded_hal::pwm::ErrorType
    for PwmChannel<TIM, C>
{
    type Error = core::convert::Infallible;
}

impl<TIM: Instance + WithPwm, const C: u8> crate::embedded_hal::pwm::SetDutyCycle
    for PwmChannel<TIM, C>
{
    fn max_duty_cycle(&self) -> u16 {
        // embedded-hal works in u16. PER + 1 only exceeds that for a TCA
        // period spanning the full 16-bit range, where true constant-high
        // output is unreachable anyway (the compare register cannot go
        // above TOP); saturating loses exactly that unreachable step.
        u16::try_from(TIM::read_period().into() + 1).unwrap_or(u16::MAX)
    }

    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
        TIM::set_compare_value_clamped(C, u32::from(duty));
        Ok(())
    }
}

pub struct PwmHz<TIM, P, PINS>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    timer: Timer<TIM>,
    pins: PINS,
    _p: PhantomData<P>,
}

impl<TIM, P, PINS> Deref for PwmHz<TIM, P, PINS>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    type Target = Timer<TIM>;
    fn deref(&self) -> &Self::Target {
        &self.timer
    }
}

impl<TIM, P, PINS> DerefMut for PwmHz<TIM, P, PINS>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.timer
    }
}

impl<TIM, P, PINS> PwmHz<TIM, P, PINS>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    /// Split into per-channel duty-cycle handles.
    ///
    /// This intentionally forfeits the pinset: the channel handles borrow
    /// nothing, so the pins stay muxed to the timer for the rest of the
    /// program. Use [`release`](Self::release) instead to tear the PWM down
    /// and recover the pins.
    pub fn split(self) -> PINS::Channels {
        PINS::split()
    }

    pub fn release(mut self) -> (Timer<TIM>, PINS) {
        self.tim.disable_counter();
        (self.timer, self.pins)
    }

    // Constructor/deconstructor pair for wrappers built outside this
    // module (the split-mode frequency-group API in tca_split.rs).
    // Unlike `release`, `into_parts` leaves the counter running.

    pub(crate) fn from_parts(timer: Timer<TIM>, pins: PINS) -> Self {
        PwmHz {
            timer,
            pins,
            _p: PhantomData,
        }
    }

    pub(crate) fn into_parts(self) -> (Timer<TIM>, PINS) {
        (self.timer, self.pins)
    }
}

impl<TIM, P, PINS> crate::traits::PwmTimer for PwmHz<TIM, P, PINS>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    type Error = Error;
    type ChannelIndex = Channel;
    type PeriodValue = Hertz;

    #[inline]
    fn enable(&mut self, channel: Self::ChannelIndex) -> Result<(), Error> {
        Ok(TIM::enable_channel(PINS::check_used(channel)? as u8, true))
    }

    #[inline]
    fn disable(&mut self, channel: Self::ChannelIndex) -> Result<(), Error> {
        Ok(TIM::enable_channel(PINS::check_used(channel)? as u8, false))
    }

    #[inline]
    fn get_duty(&self, channel: Self::ChannelIndex) -> Result<u32, Error> {
        Ok(TIM::read_compare_value(PINS::check_used(channel)? as u8).into())
    }

    #[inline]
    fn set_duty(&mut self, channel: Self::ChannelIndex, duty: u32) -> Result<(), Error> {
        Ok(TIM::set_compare_value_clamped(
            PINS::check_used(channel)? as u8,
            duty,
        ))
    }

    fn get_period(&self) -> Hertz {
        let clk = self.clk;
        let psc = self.tim.read_prescaler() as u32;
        // A PER value of N gives a period of N + 1 counter ticks; +1 exactly
        // once (the old code also added it in the divisor, reporting
        // clk / (psc * (PER + 2))).
        let per = TIM::read_period().into() + 1;

        TIM::get_input_clock_rate(clk) / (psc * per)
    }

    fn set_period(&mut self, period: Hertz) -> Result<(), Error> {
        let clk = self.clk;
        let (period, prescaler) = self
            .tim
            .calculate_period_and_prescaler::<TIM>(clk, period)?;
        if TIM::PERIOD_DOUBLE_BUFFERED {
            self.tim.set_prescaler(prescaler);
            self.tim.set_period(period)?;
            self.tim.trigger_update();
        } else {
            // An unbuffered period write is live: shrinking the top below
            // the running count would make the counter wrap through the
            // full counter width once (~65 ms at 1 MHz) before matching
            // again. Reprogramming with the counter stopped costs one
            // truncated PWM cycle instead of a runaway one.
            self.tim.disable_counter();
            self.tim.set_prescaler(prescaler);
            self.tim.set_period(period)?;
            self.tim.reset_count();
            self.tim.enable_counter();
        }
        Ok(())
    }

    #[inline]
    fn get_max_duty(&self) -> u32 {
        // PER + 1: a compare value above TOP never matches, so the output
        // stays constant-high — this is the true 100% duty value.
        TIM::read_period().into() + 1
    }

    #[inline]
    fn disable_counter(&mut self) {
        self.tim.disable_counter();
    }

    #[inline]
    fn enable_counter(&mut self) {
        self.tim.enable_counter();
    }

    #[inline]
    fn reset_count(&mut self) {
        self.tim.reset_count();
    }
}

pub struct Pwm<TIM, P, PINS, const FREQ: u32>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    timer: FTimer<TIM, FREQ>,
    pins: PINS,
    _p: PhantomData<P>,
}

impl<TIM, P, PINS, const FREQ: u32> Deref for Pwm<TIM, P, PINS, FREQ>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    type Target = FTimer<TIM, FREQ>;

    fn deref(&self) -> &Self::Target {
        &self.timer
    }
}

impl<TIM, P, PINS, const FREQ: u32> DerefMut for Pwm<TIM, P, PINS, FREQ>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.timer
    }
}

impl<TIM, P, PINS, const FREQ: u32> Pwm<TIM, P, PINS, FREQ>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    /// Split into per-channel duty-cycle handles.
    ///
    /// This intentionally forfeits the pinset: the channel handles borrow
    /// nothing, so the pins stay muxed to the timer for the rest of the
    /// program. Use [`release`](Self::release) instead to tear the PWM down
    /// and recover the pins.
    pub fn split(self) -> PINS::Channels {
        PINS::split()
    }

    pub fn release(mut self) -> (FTimer<TIM, FREQ>, PINS) {
        self.tim.disable_counter();
        (self.timer, self.pins)
    }

    // Same crate-internal pair as on `PwmHz` (see there); `into_parts`
    // leaves the counter running.

    pub(crate) fn from_parts(timer: FTimer<TIM, FREQ>, pins: PINS) -> Self {
        Pwm {
            timer,
            pins,
            _p: PhantomData,
        }
    }

    pub(crate) fn into_parts(self) -> (FTimer<TIM, FREQ>, PINS) {
        (self.timer, self.pins)
    }
}

impl<TIM, P, PINS, const FREQ: u32> crate::traits::PwmTimer for Pwm<TIM, P, PINS, FREQ>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    type Error = Error;
    type ChannelIndex = Channel;
    type PeriodValue = TimerDurationU32<FREQ>;

    #[inline]
    fn enable(&mut self, channel: Self::ChannelIndex) -> Result<(), Error> {
        Ok(TIM::enable_channel(PINS::check_used(channel)? as u8, true))
    }

    #[inline]
    fn disable(&mut self, channel: Self::ChannelIndex) -> Result<(), Error> {
        Ok(TIM::enable_channel(PINS::check_used(channel)? as u8, false))
    }

    #[inline]
    fn get_duty(&self, channel: Self::ChannelIndex) -> Result<u32, Error> {
        Ok(TIM::read_compare_value(PINS::check_used(channel)? as u8).into())
    }

    #[inline]
    fn set_duty(&mut self, channel: Self::ChannelIndex, duty: u32) -> Result<(), Error> {
        Ok(TIM::set_compare_value_clamped(
            PINS::check_used(channel)? as u8,
            duty,
        ))
    }

    fn get_period(&self) -> TimerDurationU32<FREQ> {
        TimerDurationU32::from_ticks(TIM::read_period().into() + 1)
    }

    fn set_period(&mut self, period: TimerDurationU32<FREQ>) -> Result<(), Error> {
        let period = period
            .ticks()
            .checked_sub(1)
            .ok_or(Error::ImpossiblePeriod)?
            .try_into()
            .map_err(|_| Error::ImpossiblePeriod)?;
        if TIM::PERIOD_DOUBLE_BUFFERED {
            self.tim.set_period(period)?;
            self.tim.trigger_update();
        } else {
            // Same runaway-cycle avoidance as in PwmHz::set_period above.
            self.tim.disable_counter();
            self.tim.set_period(period)?;
            self.tim.reset_count();
            self.tim.enable_counter();
        }
        Ok(())
    }

    #[inline]
    fn get_max_duty(&self) -> u32 {
        // Same PER + 1 rationale as in the PwmHz impl above.
        TIM::read_period().into() + 1
    }

    #[inline]
    fn disable_counter(&mut self) {
        self.tim.disable_counter();
    }

    #[inline]
    fn enable_counter(&mut self) {
        self.tim.enable_counter();
    }

    #[inline]
    fn reset_count(&mut self) {
        self.tim.reset_count();
    }
}

impl<TIM, P, PINS, const FREQ: u32> Pwm<TIM, P, PINS, FREQ>
where
    TIM: Instance + WithPwm,
    PINS: Pins<TIM, P>,
{
    #[inline]
    pub fn get_duty_time(&self, channel: Channel) -> Result<TimerDurationU32<FREQ>, Error> {
        Ok(TimerDurationU32::from_ticks(
            TIM::read_compare_value(PINS::check_used(channel)? as u8).into(),
        ))
    }

    #[inline]
    pub fn set_duty_time(
        &mut self,
        channel: Channel,
        duty: TimerDurationU32<FREQ>,
    ) -> Result<(), Error> {
        // A duty duration longer than the period is not an error here: the
        // clamped write yields a constant-high output, consistent with the
        // u32 duty domain semantics.
        Ok(TIM::set_compare_value_clamped(
            PINS::check_used(channel)? as u8,
            duty.ticks(),
        ))
    }
}

impl<TIM: Instance + WithPwm> Timer<TIM> {
    pub fn pwm_hz<P, PINS>(
        mut self,
        pins: PINS,
        freq: Hertz,
        mode: TIM::GenerationMode,
    ) -> Result<PwmHz<TIM, P, PINS>, Error>
    where
        PINS: Pins<TIM, P>,
    {
        // This constructor programs PER from the requested frequency, which
        // only works for generation modes whose TOP is PER.
        if !TIM::is_period_driven(&mode) {
            return Err(Error::UnsupportedPwmMode);
        }

        self.tim.disable_counter();
        self.tim.reset_count();
        // Select the clock source the period math below assumes — without
        // this a TCB configured for CLKTCA silently ran from CLK_PER.
        self.tim.prepare_clock_source(self.clk);
        self.tim.set_pwm_mode(mode);
        self.tim.clear_overflow();

        let (period, prescaler) = self
            .tim
            .calculate_period_and_prescaler::<TIM>(self.clk, freq)?;
        self.tim.set_prescaler(prescaler);
        self.tim.set_period(period)?;
        self.tim.trigger_update();

        self.tim.enable_counter();

        Ok(PwmHz {
            timer: self,
            pins,
            _p: PhantomData,
        })
    }
}

impl<TIM: Instance + WithPwm> Timer<TIM> {
    pub fn pwm_custom<P, PINS>(
        mut self,
        pins: PINS,
        prescaler: u16,
        period: TIM::CounterValue,
        mode: TIM::GenerationMode,
    ) -> Result<PwmHz<TIM, P, PINS>, Error>
    where
        PINS: Pins<TIM, P>,
    {
        // Same period-driven-mode requirement as in pwm_hz above.
        if !TIM::is_period_driven(&mode) {
            return Err(Error::UnsupportedPwmMode);
        }

        self.tim.disable_counter();
        self.tim.reset_count();
        // Same clock-source selection as in pwm_hz above.
        self.tim.prepare_clock_source(self.clk);
        self.tim.set_pwm_mode(mode);
        self.tim.clear_overflow();

        let prescaler = TIM::get_valid_prescalers(self.clk)
            .iter()
            .find(|e| **e == prescaler)
            .ok_or(Error::ImpossiblePrescaler)?;
        self.tim.set_prescaler(*prescaler);
        self.tim.set_period(period)?;
        self.tim.trigger_update();

        self.tim.enable_counter();

        Ok(PwmHz {
            timer: self,
            pins,
            _p: PhantomData,
        })
    }
}

impl<TIM: Instance + WithPwm, const FREQ: u32> FTimer<TIM, FREQ> {
    pub fn pwm<P, PINS>(
        mut self,
        pins: PINS,
        time: TimerDurationU32<FREQ>,
        mode: TIM::GenerationMode,
    ) -> Result<Pwm<TIM, P, PINS, FREQ>, Error>
    where
        PINS: Pins<TIM, P>,
    {
        // Same period-driven-mode requirement as in Timer::pwm_hz.
        if !TIM::is_period_driven(&mode) {
            return Err(Error::UnsupportedPwmMode);
        }

        // We are an FTimer, so at this point the clock source and prescaler
        // are already set up based on the target frequency in FREQ

        self.tim.disable_counter();
        self.tim.reset_count();
        self.tim.set_pwm_mode(mode);
        self.tim.clear_overflow();

        let period = time
            .ticks()
            .checked_sub(1)
            .ok_or(Error::ImpossiblePeriod)?
            .try_into()
            .map_err(|_| Error::ImpossiblePeriod)?;
        self.tim.set_period(period)?;
        self.tim.trigger_update();

        self.tim.enable_counter();

        Ok(Pwm {
            timer: self,
            pins,
            _p: PhantomData,
        })
    }
}
