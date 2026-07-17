//! # Configurable Custom Logic

use crate::pac::{
    ccl::lut::{lutctrla, lutctrlb, lutctrlc},
    ccl::seqctrl,
    CCL,
};
use crate::Toggle;

use core::marker::PhantomData;

// TODO: allow config of RUNSTDBY

/// CCL Lookup table Output pin
pub trait OutputPin<LUT>: crate::private::Sealed {}

/// CCL Lookup table Input pin
pub trait InputPin<LUT, const IDX: u8>: crate::private::Sealed {}

/// Pin set for the port multiplexer
pub struct CclLutOutputPinset<LUT, Out: OutputPin<LUT>> {
    _lut: PhantomData<LUT>,
    out: Out,
}

impl<LUT, Out> CclLutOutputPinset<LUT, Out>
where
    Out: OutputPin<LUT>,
{
    pub(crate) fn new(out: Out) -> Self {
        CclLutOutputPinset {
            _lut: PhantomData,
            out,
        }
    }

    pub fn free(self) -> Out {
        self.out
    }
}

/// Extension trait to configure a `CCL` peripheral and all containing LUTs
pub trait CclExt {
    /// The Parts to split the `CCL` peripheral into
    type Parts;

    /// Splits the `CCL` block into independent LUTs
    fn split(self) -> Self::Parts;
}

/// CCL Register interface traits private to this module
mod private {
    use super::{ClockSource, FilterSelection, Input0, Input1, Input2, SequencerConfig, Toggle};

    pub trait CclRegExt {
        fn enable(&self);
        fn disable(&self);
        fn sequencer_config(&self, seq_idx: u8, config: SequencerConfig);

        fn lut_edge_detection(&self, i: u8, enable: Toggle);
        fn lut_output_enable(&self, i: u8, enable: Toggle);
        fn lut_filter_selection(&self, lut_idx: u8, filter: FilterSelection);
        fn lut_clock_source(&self, lut_idx: u8, filter: ClockSource);
        fn lut_enable(&self, lut_idx: u8, state: Toggle);
        fn lut_inputs(&self, lut_idx: u8, input0: Input0, input1: Input1, input2: Input2);
        fn lut_table(&self, lut_idx: u8, table: u8);
    }

    pub trait Ccl {
        type Reg: CclRegExt + ?Sized;

        fn ptr(&self) -> *const Self::Reg;
    }
}

/// Marker traits used in this module
pub mod marker {
    /// Marker trait for CCLs
    pub trait Ccl: super::private::Ccl {}

    /// Marker trait for unconfigured LUT
    pub trait Disabled {}

    /// Marker trait for configured and enabled LUT
    pub trait Enabled {}

    /// Marker trait for LUT index
    pub trait Index {
        #[doc(hidden)]
        fn index(&self) -> u8;
    }
}

/// Runtime defined LUT index (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Ux(u8);

impl marker::Index for Ux {
    fn index(&self) -> u8 {
        self.0
    }
}

/// Compile time defined LUT index (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct U<const X: u8>;

impl<const X: u8> marker::Index for U<X> {
    #[inline(always)]
    fn index(&self) -> u8 {
        X
    }
}

/// Active LUT (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Active;

/// Inactive LUT (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Inactive;

impl marker::Enabled for Active {}
impl marker::Disabled for Inactive {}

/// Generic LUT
#[derive(Debug)]
pub struct Lut<Ccl, Index, State> {
    pub(crate) ccl: Ccl,
    pub(crate) index: Index,
    _state: PhantomData<State>,
}

// Make all LUT peripheral trait extensions sealable.
impl<Ccl, Index, State> crate::private::Sealed for Lut<Ccl, Index, State> {}

/// Fully erased LUT
///
/// This moves the LUT type information to be known
/// at runtime, and erases the specific compile time type of the LUT.
/// The only compile time information of the LUT is its Mode.
pub type Lutx<Ccl, State> = Lut<Ccl, Ux, State>;

impl<Ccl, State, const X: u8> Lut<Ccl, U<X>, State> {
    /// Erases the pin number from the type
    ///
    /// This is useful when you want to collect the pins into an array where you
    /// need all the elements to have the same type
    pub fn downgrade(self) -> Lut<Ccl, Ux, State> {
        Lut {
            ccl: self.ccl,
            index: Ux(X),
            _state: self._state,
        }
    }
}

impl<Ccl, Index, State> Lut<Ccl, Index, State> {
    fn into_state<NewState>(self) -> Lut<Ccl, Index, NewState> {
        Lut {
            ccl: self.ccl,
            index: self.index,
            _state: PhantomData,
        }
    }
}

// Every LUT method — including enable/disable — takes a proof that the whole
// CCL block is disabled: due to the errata acknowledged on
// [`Control::enable`], *all* LUT and SEQCTRL registers are enable-protected
// while `CTRLA.ENABLE` is set, so any write issued in that window is silently
// discarded by the hardware.
impl<Ccl, Index> Lut<Ccl, Index, Inactive>
where
    Ccl: marker::Ccl,
    Index: marker::Index,
{
    /// Enable the LUT.
    ///
    /// An enabled LUT cannot be reconfigured until it's disabled again using
    /// [`Lut::disable`].
    #[inline]
    pub fn enable(self, _ctrl: &Control<Ccl, Inactive>) -> Lut<Ccl, Index, Active> {
        unsafe { (*self.ccl.ptr()).lut_enable(self.index.index(), Toggle::On) };
        self.into_state()
    }

    /// Configure the edge detection.
    #[inline]
    pub fn edge_detection(self, _ctrl: &Control<Ccl, Inactive>, enable: Toggle) -> Self {
        unsafe { (*self.ccl.ptr()).lut_edge_detection(self.index.index(), enable) };
        self
    }

    /// Configure the output pin of the LUT.
    ///
    /// When enabled, this overrides the pin configuration of the
    /// PORT I/O controller.
    #[inline]
    pub fn output_enable(self, _ctrl: &Control<Ccl, Inactive>, enable: Toggle) -> Self {
        unsafe { (*self.ccl.ptr()).lut_output_enable(self.index.index(), enable) };
        self
    }

    /// Configure the output synchronization filter.
    #[inline]
    pub fn filter(self, _ctrl: &Control<Ccl, Inactive>, filter: FilterSelection) -> Self {
        unsafe { (*self.ccl.ptr()).lut_filter_selection(self.index.index(), filter) };
        self
    }

    /// Select the clock source for the sequencer.
    #[inline]
    pub fn clock_source(self, _ctrl: &Control<Ccl, Inactive>, clock_src: ClockSource) -> Self {
        unsafe { (*self.ccl.ptr()).lut_clock_source(self.index.index(), clock_src) };
        self
    }

    /// Define the two inputs into the lookup table.
    #[inline]
    pub fn inputs(
        self,
        _ctrl: &Control<Ccl, Inactive>,
        input0: Input0,
        input1: Input1,
        input2: Input2,
    ) -> Self {
        unsafe { (*self.ccl.ptr()).lut_inputs(self.index.index(), input0, input1, input2) };
        self
    }

    /// Set the lookup table value.
    #[inline]
    pub fn table(self, _ctrl: &Control<Ccl, Inactive>, lookup_table: u8) -> Self {
        unsafe { (*self.ccl.ptr()).lut_table(self.index.index(), lookup_table) };
        self
    }
}

impl<Ccl, Index> Lut<Ccl, Index, Active>
where
    Ccl: marker::Ccl,
    Index: marker::Index,
{
    /// Disable the LUT
    ///
    /// A disabled LUT can be reconfigured again until it's enabled using
    /// [`Lut::enable`].
    #[inline]
    pub fn disable(self, _ctrl: &Control<Ccl, Inactive>) -> Lut<Ccl, Index, Inactive> {
        unsafe { (*self.ccl.ptr()).lut_enable(self.index.index(), Toggle::Off) };
        self.into_state()
    }
}

use private::CclRegExt;

impl CclRegExt for crate::pac::ccl::RegisterBlock {
    #[inline(always)]
    fn enable(&self) {
        // modify, not write: a plain write would clobber RUNSTDBY once
        // configuring it is supported (see the TODO at the top of the file).
        self.ctrla().modify(|_, w| w.enable().set_bit());
    }

    #[inline(always)]
    fn disable(&self) {
        self.ctrla().modify(|_, w| w.enable().clear_bit());
    }

    #[inline(always)]
    fn sequencer_config(&self, seq_idx: u8, config: SequencerConfig) {
        self.seqctrl(seq_idx as usize)
            .write(|w| w.seqsel().variant(config.into()));
    }

    #[inline(always)]
    fn lut_edge_detection(&self, lut_idx: u8, enable: Toggle) {
        self.lut(lut_idx as usize)
            .lutctrla()
            .modify(|_, w| w.edgedet().bit(enable.into()));
    }

    #[inline(always)]
    fn lut_output_enable(&self, lut_idx: u8, enable: Toggle) {
        self.lut(lut_idx as usize)
            .lutctrla()
            .modify(|_, w| w.outen().bit(enable.into()));
    }

    #[inline(always)]
    fn lut_filter_selection(&self, lut_idx: u8, filter: FilterSelection) {
        self.lut(lut_idx as usize)
            .lutctrla()
            .modify(|_, w| w.filtsel().variant(filter.into()));
    }

    #[inline(always)]
    fn lut_clock_source(&self, lut_idx: u8, filter: ClockSource) {
        // `.bit()` instead of `.variant()`: CLKSRC is a plain bit in most
        // PACs, but an enumerated single-bit field (CLKPER/IN2) in the ones
        // generated from newer ATDF packs. `.bit()` compiles against both.
        self.lut(lut_idx as usize)
            .lutctrla()
            .modify(|_, w| w.clksrc().bit(filter.into()));
    }

    #[inline(always)]
    fn lut_enable(&self, lut_idx: u8, state: Toggle) {
        self.lut(lut_idx as usize)
            .lutctrla()
            .modify(|_, w| w.enable().variant(state.into()));
    }

    #[inline(always)]
    fn lut_inputs(&self, lut_idx: u8, input0: Input0, input1: Input1, input2: Input2) {
        self.lut(lut_idx as usize).lutctrlb().modify(|_, w| {
            w.insel0()
                .variant(input0.into())
                .insel1()
                .variant(input1.into())
        });
        self.lut(lut_idx as usize)
            .lutctrlc()
            .modify(|_, w| w.insel2().variant(input2.into()));
    }

    #[inline(always)]
    fn lut_table(&self, lut_idx: u8, table: u8) {
        self.lut(lut_idx as usize).truth().write(|w| w.set(table));
    }
}

/// Generic main control block for a CCL
///
/// The `State` type parameter tracks whether the CCL-wide `CTRLA.ENABLE` bit
/// is set. All LUT and sequencer configuration requires a
/// `&Control<_, Inactive>` because of the enable-protection errata described
/// on [`Control::enable`].
// Debug only, no uDebug: ufmt provides no uDebug impl for PhantomData, same
// as on [`Lut`].
#[derive(Debug)]
pub struct Control<Ccl, State = Inactive> {
    pub(crate) ccl: Ccl,
    _state: PhantomData<State>,
}

// Make all Control peripheral trait extensions sealable.
impl<Ccl, State> crate::private::Sealed for Control<Ccl, State> {}

impl<Ccl> Control<Ccl, Inactive>
where
    Ccl: marker::Ccl,
{
    /// Enable the CCL peripheral block
    ///
    /// NOTE: Due to an errata, the whole CCL block needs to be disabled
    /// completely to reconfigure even independent LUTs, otherwise registers
    /// in the LUT region are still going to be enable-protected. The AVR-DD
    /// series fixes this errata. This is why LUT and sequencer configuration
    /// requires a reference to a `Control<_, Inactive>`.
    #[inline]
    pub fn enable(self) -> Control<Ccl, Active> {
        unsafe { (*self.ccl.ptr()).enable() };
        Control {
            ccl: self.ccl,
            _state: PhantomData,
        }
    }

    /// Set the sequencer config to connect multiple LUTs together and build
    /// feedback loops
    #[inline]
    pub fn sequencer_config(&self, seq: Sequencer, cfg: SequencerConfig) {
        unsafe { (*self.ccl.ptr()).sequencer_config(seq.into(), cfg) };
    }
}

impl<Ccl> Control<Ccl, Active>
where
    Ccl: marker::Ccl,
{
    /// Disable the CCL peripheral block, making LUT and sequencer
    /// configuration possible again
    #[inline]
    pub fn disable(self) -> Control<Ccl, Inactive> {
        unsafe { (*self.ccl.ptr()).disable() };
        Control {
            ccl: self.ccl,
            _state: PhantomData,
        }
    }
}

/// The CCL itself (type state)
pub struct Ccl;

impl private::Ccl for Ccl {
    type Reg = crate::pac::ccl::RegisterBlock;

    fn ptr(&self) -> *const Self::Reg {
        CCL::ptr()
    }
}

impl marker::Ccl for Ccl {}

macro_rules! ccl {
    ({
        luts: [$(
            {
                lut: $index:literal,
            },
        )+],
    }) => {
        paste::paste! {
            $(
                #[doc = concat!("Lookup table ", stringify!([<LUT $index>]))]
                pub type [<LUT $index>] = Lut<Ccl, U<$index>, Inactive>;
            )+

            /// CCL Parts
            pub struct Parts {
                pub control: Control<Ccl>,
                $(
                    pub [<lut $index>]: [<LUT $index>],
                )+
            }

            impl CclExt for CCL {
                type Parts = Parts;

                fn split(self) -> Self::Parts {
                    Self::Parts {
                        control: Control { ccl: Ccl, _state: PhantomData },
                        $(
                            [<lut $index>]: [<LUT $index>] {
                                ccl: Ccl,
                                index: U::<$index>,
                                _state: PhantomData,
                            },
                        )+
                    }
                }
            }
        }
    };
}

ccl!({
    luts: [
        { lut: 0, },
        { lut: 1, },
    ],
});

// FIXME: below structs are all device-dependent

#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sequencer {
    // The whole 0/1-series has exactly two LUTs and thus one sequencer.
    // Sequencers for LUT23/LUT45 only appear on the 2-series and the
    // AVR Dx families.
    LUT01,
}

impl From<Sequencer> for u8 {
    fn from(value: Sequencer) -> Self {
        match value {
            Sequencer::LUT01 => 0,
            //Sequencer::LUT23 => 1,
            //Sequencer::LUT45 => 2,
        }
    }
}

#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerConfig {
    Disable,
    DFlipFlop,
    JKFlipFlop,
    DLatch,
    RSLatch,
}

impl From<SequencerConfig> for seqctrl::SEQSEL_A {
    fn from(value: SequencerConfig) -> Self {
        match value {
            SequencerConfig::Disable => seqctrl::SEQSEL_A::DISABLE,
            SequencerConfig::DFlipFlop => seqctrl::SEQSEL_A::DFF,
            SequencerConfig::JKFlipFlop => seqctrl::SEQSEL_A::JK,
            SequencerConfig::DLatch => seqctrl::SEQSEL_A::LATCH,
            SequencerConfig::RSLatch => seqctrl::SEQSEL_A::RS,
        }
    }
}

#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterSelection {
    Disable,
    SynchronizerEnabled,
    FilterEnabled,
}

impl From<FilterSelection> for lutctrla::FILTSEL_A {
    fn from(value: FilterSelection) -> Self {
        match value {
            FilterSelection::Disable => lutctrla::FILTSEL_A::DISABLE,
            FilterSelection::SynchronizerEnabled => lutctrla::FILTSEL_A::SYNCH,
            FilterSelection::FilterEnabled => lutctrla::FILTSEL_A::FILTER,
        }
    }
}

#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSource {
    PeripheralClock,
    Input2,
}

impl From<ClockSource> for bool {
    fn from(value: ClockSource) -> Self {
        match value {
            ClockSource::PeripheralClock => false,
            ClockSource::Input2 => true,
        }
    }
}

// The INSEL encodings are identical across the whole 0/1-series (the TCD0
// selections exist in the 0-series register maps too, even though those
// chips have no TCD0 — selecting one there just yields a constant input).
// The 16 KB+ 1-series parts append AC1/AC2/TCB1 selections.

pub enum Input0 {
    Masked,
    Feedback,
    Link,
    Event01,
    Event23,
    IoPin,
    Ac0Out,
    Tcb0Wo,
    Tca0Wo0,
    Tcd0Woa,
    Usart0Xck,
    Spi0Sck,
    #[cfg(feature = "periph-ac1")]
    Ac1Out,
    #[cfg(feature = "periph-ac2")]
    Ac2Out,
    #[cfg(feature = "periph-tcb1")]
    Tcb1Wo,
}

impl From<Input0> for lutctrlb::INSEL0_A {
    fn from(input: Input0) -> Self {
        match input {
            Input0::Masked => lutctrlb::INSEL0_A::MASK,
            Input0::Feedback => lutctrlb::INSEL0_A::FEEDBACK,
            Input0::Link => lutctrlb::INSEL0_A::LINK,
            Input0::Event01 => lutctrlb::INSEL0_A::EVENT0,
            Input0::Event23 => lutctrlb::INSEL0_A::EVENT1,
            Input0::IoPin => lutctrlb::INSEL0_A::IO,
            Input0::Ac0Out => lutctrlb::INSEL0_A::AC0,
            Input0::Tcb0Wo => lutctrlb::INSEL0_A::TCB0,
            Input0::Tca0Wo0 => lutctrlb::INSEL0_A::TCA0,
            Input0::Tcd0Woa => lutctrlb::INSEL0_A::TCD0,
            Input0::Usart0Xck => lutctrlb::INSEL0_A::USART0,
            Input0::Spi0Sck => lutctrlb::INSEL0_A::SPI0,
            #[cfg(feature = "periph-ac1")]
            Input0::Ac1Out => lutctrlb::INSEL0_A::AC1,
            #[cfg(feature = "periph-ac2")]
            Input0::Ac2Out => lutctrlb::INSEL0_A::AC2,
            #[cfg(feature = "periph-tcb1")]
            Input0::Tcb1Wo => lutctrlb::INSEL0_A::TCB1,
        }
    }
}

pub enum Input1 {
    Masked,
    Feedback,
    Link,
    Event01,
    Event23,
    IoPin,
    Ac0Out,
    Tcb0Wo,
    Tca0Wo1,
    Tcd0Wob,
    Usart0Txd,
    Spi0Mosi,
    #[cfg(feature = "periph-ac1")]
    Ac1Out,
    #[cfg(feature = "periph-ac2")]
    Ac2Out,
    #[cfg(feature = "periph-tcb1")]
    Tcb1Wo,
}

impl From<Input1> for lutctrlb::INSEL1_A {
    fn from(input: Input1) -> Self {
        match input {
            Input1::Masked => lutctrlb::INSEL1_A::MASK,
            Input1::Feedback => lutctrlb::INSEL1_A::FEEDBACK,
            Input1::Link => lutctrlb::INSEL1_A::LINK,
            Input1::Event01 => lutctrlb::INSEL1_A::EVENT0,
            Input1::Event23 => lutctrlb::INSEL1_A::EVENT1,
            Input1::IoPin => lutctrlb::INSEL1_A::IO,
            Input1::Ac0Out => lutctrlb::INSEL1_A::AC0,
            Input1::Tcb0Wo => lutctrlb::INSEL1_A::TCB0,
            Input1::Tca0Wo1 => lutctrlb::INSEL1_A::TCA0,
            Input1::Tcd0Wob => lutctrlb::INSEL1_A::TCD0,
            Input1::Usart0Txd => lutctrlb::INSEL1_A::USART0,
            Input1::Spi0Mosi => lutctrlb::INSEL1_A::SPI0,
            #[cfg(feature = "periph-ac1")]
            Input1::Ac1Out => lutctrlb::INSEL1_A::AC1,
            #[cfg(feature = "periph-ac2")]
            Input1::Ac2Out => lutctrlb::INSEL1_A::AC2,
            #[cfg(feature = "periph-tcb1")]
            Input1::Tcb1Wo => lutctrlb::INSEL1_A::TCB1,
        }
    }
}

pub enum Input2 {
    Masked,
    Feedback,
    Link,
    Event0,
    Event1,
    IoPin,
    Ac0Out,
    Tcb0Wo,
    Tca0Wo2,
    Tcd0Woa,
    Spi0Miso,
    #[cfg(feature = "periph-ac1")]
    Ac1Out,
    #[cfg(feature = "periph-ac2")]
    Ac2Out,
    #[cfg(feature = "periph-tcb1")]
    Tcb1Wo,
}

impl From<Input2> for lutctrlc::INSEL2_A {
    fn from(input: Input2) -> Self {
        match input {
            Input2::Masked => lutctrlc::INSEL2_A::MASK,
            Input2::Feedback => lutctrlc::INSEL2_A::FEEDBACK,
            Input2::Link => lutctrlc::INSEL2_A::LINK,
            Input2::Event0 => lutctrlc::INSEL2_A::EVENT0,
            Input2::Event1 => lutctrlc::INSEL2_A::EVENT1,
            Input2::IoPin => lutctrlc::INSEL2_A::IO,
            Input2::Ac0Out => lutctrlc::INSEL2_A::AC0,
            Input2::Tcb0Wo => lutctrlc::INSEL2_A::TCB0,
            Input2::Tca0Wo2 => lutctrlc::INSEL2_A::TCA0,
            Input2::Tcd0Woa => lutctrlc::INSEL2_A::TCD0,
            Input2::Spi0Miso => lutctrlc::INSEL2_A::SPI0,
            #[cfg(feature = "periph-ac1")]
            Input2::Ac1Out => lutctrlc::INSEL2_A::AC1,
            #[cfg(feature = "periph-ac2")]
            Input2::Ac2Out => lutctrlc::INSEL2_A::AC2,
            #[cfg(feature = "periph-tcb1")]
            Input2::Tcb1Wo => lutctrlc::INSEL2_A::TCB1,
        }
    }
}

// TODO: I didn't manage yet to add pins to the LUT state so far
//
// Pin tables per package (datasheet I/O-multiplexing chapter): LUT0's
// inputs (PA0-PA2) exist everywhere; its default output is PA4, except on
// the 8-pin die where it is PA6. LUT1's output is PA7 everywhere, and it
// has no input pins below the 20-pin package (PC3, plus PC4/PC5 on
// 24-pin). The alternate outputs (PB4/PC1) only exist on 20/24-pin
// packages.
use crate::gpio::Input;

// Each entry also generates the PORTMUX `IntoMuxedPinset` impl (`clear` =
// default position, `set` = alternate).
use crate::portmux::single_output_pins;

single_output_pins! {
    key: LUT0,
    marker: OutputPin,
    pinset: CclLutOutputPinset,
    token: Lut0Mux,
    route: ctrla / lut0,
    pins: [
        #[cfg(feature = "pins-8")]
        (A/a, 6) => clear,
        #[cfg(not(feature = "pins-8"))]
        (A/a, 4) => clear,
        #[cfg(any(feature = "pins-20", feature = "pins-24"))]
        (B/b, 4) => set,
    ]
}

single_output_pins! {
    key: LUT1,
    marker: OutputPin,
    pinset: CclLutOutputPinset,
    token: Lut1Mux,
    route: ctrla / lut1,
    pins: [
        (A/a, 7) => clear,
        #[cfg(any(feature = "pins-20", feature = "pins-24"))]
        (C/c, 1) => set,
    ]
}

impl InputPin<LUT0, 0> for crate::gpio::porta::PA0<Input> {}
impl InputPin<LUT0, 1> for crate::gpio::porta::PA1<Input> {}
impl InputPin<LUT0, 2> for crate::gpio::porta::PA2<Input> {}

#[cfg(any(feature = "pins-20", feature = "pins-24"))]
impl InputPin<LUT1, 0> for crate::gpio::portc::PC3<Input> {}
#[cfg(feature = "pins-24")]
impl InputPin<LUT1, 1> for crate::gpio::portc::PC4<Input> {}
#[cfg(feature = "pins-24")]
impl InputPin<LUT1, 2> for crate::gpio::portc::PC5<Input> {}

use crate::evsys::ChannelConfigurator;
use crate::evsys::{Channel, EventGenerator, GeneratorAssigned, Unconfigured};

impl<Evsys, Index, C, State, const X: u8> EventGenerator<Evsys, crate::evsys::Async, Index>
    for Lut<C, U<X>, State>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Async, Index, Unconfigured>,
        _source: (),
    ) -> Channel<Evsys, crate::evsys::Async, Index, GeneratorAssigned> {
        channel.set_generator(0x01 + X);
        channel.with_state(GeneratorAssigned)
    }
}
