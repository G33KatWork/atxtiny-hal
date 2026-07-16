//! # Event system

use core::marker::PhantomData;

/// Extension trait to configure an `EVSYS` peripheral and all containing channels
pub trait EvsysExt {
    /// The Parts to split the `EVSYS` peripheral into
    type Parts;

    /// Splits the EVSYS block into independent channels
    fn split(self) -> Self::Parts;
}

/// `EVSYS` Register interface traits private to this module
mod private {
    pub trait EvsysRegExt {
        fn set_async_generator(&self, channel_idx: u8, generator: u8);
        fn set_sync_generator(&self, channel_idx: u8, generator: u8);

        // `user_reg_index` selects the ASYNCUSERn/SYNCUSERn register,
        // `channel_select` is the value written into it. The old parameter
        // names had these two roles swapped, which directly produced the
        // free_user corruption.
        fn set_async_user(&self, user_reg_index: u8, channel_select: u8);
        fn set_sync_user(&self, user_reg_index: u8, channel_select: u8);

        //FIXME: add strobes
    }

    pub trait Evsys {
        type Reg: EvsysRegExt + ?Sized;

        fn ptr(&self) -> *const Self::Reg;
    }
}

use private::EvsysRegExt;

/// Marker traits used in this module
pub mod marker {
    /// Marker trait for event systems
    pub trait Evsys: super::private::Evsys {}

    /// Marker trait for the flavor of a channel (Synchronous vs Asynchronous)
    pub trait ChannelFlavor {}

    /// Marker trait for the state of a channel
    pub trait ChannelState {}

    /// Marker trait for channel indexes
    pub trait Index {
        type const X: u8;
        type const UX: u8;

        #[doc(hidden)]
        fn index(&self) -> u8;
    }
}

/// Compile time defined channel index (type state)
#[derive(Debug, Default)]
pub struct U<const X: u8, const UX: u8>;

impl<const X: u8, const UX: u8> marker::Index for U<X, UX> {
    type const X: u8 = X;
    type const UX: u8 = UX;

    #[inline(always)]
    fn index(&self) -> u8 {
        X
    }
}

/// Asynchronous channel (type state)
#[derive(ufmt::derive::uDebug, Debug, Default)]
pub struct Async;

/// Synchronous channel (type state)
#[derive(ufmt::derive::uDebug, Debug, Default)]
pub struct Sync;

/// Unconfigured channel (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Unconfigured;

/// Generator assigned to channel (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct GeneratorAssigned;

/// Fully configured channel where generator and user is assigned (type state)
///
/// Holds the connected user's token, which is what lets
/// [`free_user`](Channel::free_user) clear the user register that was
/// actually connected and hand the token back for reuse.
pub struct Configured<User> {
    user: User,
}

// Manual impl instead of derive to avoid a needless `User: Debug` bound.
impl<User> core::fmt::Debug for Configured<User> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Configured")
    }
}

impl marker::ChannelFlavor for Async {}
impl marker::ChannelFlavor for Sync {}

impl marker::ChannelState for Unconfigured {}
impl marker::ChannelState for GeneratorAssigned {}
impl<User> marker::ChannelState for Configured<User> {}

/// Generic event channel
#[derive(Debug)]
pub struct Channel<Evsys, Flavor, Index, State> {
    pub(crate) evsys: Evsys,
    pub(crate) index: Index,
    pub(crate) state: State,
    pub(crate) _phantom: PhantomData<Flavor>,
}

impl<Evsys, Flavor, Index, State> crate::private::Sealed for Channel<Evsys, Flavor, Index, State> {}

impl<Evsys, Flavor, Index, State> Channel<Evsys, Flavor, Index, State> {
    pub(crate) fn with_state<NewState>(self, state: NewState) -> Channel<Evsys, Flavor, Index, NewState> {
        Channel {
            evsys: self.evsys,
            index: self.index,
            state,
            _phantom: PhantomData,
        }
    }
}

macro_rules! evsys {
    ({
        channels: [$(
            {
                channel: $index:literal,
                userindex: $userindex:literal,
                flavor: $flavor:ty,
                // generators: {
                //     $($generator:ident => $value:literal,)+
                // }
            },
        )+],
    }) => {
        paste::paste! {
            /// Event system
            #[derive(ufmt::derive::uDebug, Debug)]
            pub struct Evsys;

            impl marker::Evsys for Evsys {}

            impl private::Evsys for Evsys {
                type Reg = crate::pac::evsys::RegisterBlock;

                fn ptr(&self) -> *const Self::Reg {
                    crate::pac::EVSYS::ptr()
                }
            }

            impl EvsysRegExt for crate::pac::evsys::RegisterBlock {
                fn set_async_generator(&self, channel_idx: u8, generator: u8) {
                    self.asyncch(channel_idx as usize).write(|f| unsafe { f.bits(generator) });
                }

                fn set_sync_generator(&self, channel_idx: u8, generator: u8) {
                    self.syncch(channel_idx as usize).write(|f| unsafe { f.bits(generator) });
                }

                fn set_async_user(&self, user_reg_index: u8, channel_select: u8) {
                    self.asyncuser(user_reg_index as usize).write(|f| unsafe { f.bits(channel_select) });
                }

                fn set_sync_user(&self, user_reg_index: u8, channel_select: u8) {
                    self.syncuser(user_reg_index as usize).write(|f| unsafe { f.bits(channel_select) });
                }
            }

            $(
                #[doc = concat!("Event channel ", stringify!($index))]
                pub type [<Channel $flavor $index>] = Channel<Evsys, $flavor, U<$index, $userindex>, Unconfigured>;

                // #[doc = concat!("Event channel ", stringify!($index), " generator sources")]
                // #[allow(non_camel_case_types)]
                // #[repr(u8)]
                // pub enum [<Channel $flavor $index Generator>] {
                //     $(
                //         $generator = $value,
                //     )+
                // }
            )+

            /// EVSYS Parts
            pub struct Parts {
                $(
                    pub [<channel_ $flavor:lower $index>]: [<Channel $flavor $index>],
                )+

                /// Event user token for the TCA0 event input (SYNCUSER0)
                pub user_tca0: UserTca0,

                /// Event user token for the USART0 IrDA event input (SYNCUSER1)
                pub user_usart0: UserUsart0,
            }

            impl EvsysExt for crate::pac::EVSYS {
                type Parts = Parts;

                fn split(self) -> Self::Parts {
                    Self::Parts {
                        $(
                            [<channel_ $flavor:lower $index>]: [<Channel $flavor $index>] {
                                evsys: Evsys,
                                index: U::<$index, $userindex>::default(),
                                state: Unconfigured,
                                _phantom: PhantomData::default(),
                            },
                        )+
                        user_tca0: UserTca0 { _private: () },
                        user_usart0: UserUsart0 { _private: () },
                    }
                }
            }
        }
    };
}

/// The register file an event user's [`MULTIPLEXER_INDEX`] points into
///
/// The EVSYS has two independent user-register files, ASYNCUSERn and
/// SYNCUSERn. Sync channels are selectable from *both* files (select
/// values 1/2 = SYNCCH0/1 in either encoding); async channels only from
/// the async file.
///
/// [`MULTIPLEXER_INDEX`]: EventUser::MULTIPLEXER_INDEX
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRegisterFile {
    /// The user register is one of ASYNCUSERn
    Async,
    /// The user register is one of SYNCUSERn
    Sync,
}

/// An event user reachable from channels of the given `Flavor`
///
/// Users living in the async register file can be driven by async *and*
/// sync channels (they implement this trait for both flavors); users in
/// the sync file only by sync channels.
///
/// Sealed: a foreign impl with an out-of-range `MULTIPLEXER_INDEX` would
/// panic on the user-register array access.
pub trait EventUser<Evsys, Flavor>: crate::private::Sealed
where
    Evsys: marker::Evsys,
    Flavor: marker::ChannelFlavor,
{
    /// Index of this user's register within [`Self::FILE`]
    const MULTIPLEXER_INDEX: u8;

    /// The user-register file [`Self::MULTIPLEXER_INDEX`] refers to
    const FILE: UserRegisterFile;
}

impl<Evsys, Flavor, Index, State> Channel<Evsys, Flavor, Index, State>
where
    Evsys: marker::Evsys,
{
    fn write_user(&mut self, file: UserRegisterFile, user_reg_index: u8, channel_select: u8) {
        match file {
            UserRegisterFile::Async => unsafe {
                (*self.evsys.ptr()).set_async_user(user_reg_index, channel_select)
            },
            UserRegisterFile::Sync => unsafe {
                (*self.evsys.ptr()).set_sync_user(user_reg_index, channel_select)
            },
        }
    }
}

impl<Evsys, Flavor, Index> Channel<Evsys, Flavor, Index, GeneratorAssigned>
where
    Evsys: marker::Evsys,
    Flavor: marker::ChannelFlavor,
    Index: marker::Index,
{
    /// Connect this channel to an event user, consuming the user's token
    ///
    /// Consuming the token makes a second, conflicting connection of the
    /// same user unrepresentable; [`free_user`](Channel::free_user) hands
    /// the token back.
    pub fn connect_event_user<U: EventUser<Evsys, Flavor>>(
        mut self,
        user: U,
    ) -> Channel<Evsys, Flavor, Index, Configured<U>> {
        self.write_user(U::FILE, U::MULTIPLEXER_INDEX, Index::UX);
        self.with_state(Configured { user })
    }
}

impl<Evsys, Flavor, Index, User> Channel<Evsys, Flavor, Index, Configured<User>>
where
    Evsys: marker::Evsys,
    Flavor: marker::ChannelFlavor,
    Index: marker::Index,
    User: EventUser<Evsys, Flavor>,
{
    /// Disconnect the connected event user and hand its token back
    pub fn free_user(mut self) -> (Channel<Evsys, Flavor, Index, GeneratorAssigned>, User) {
        // Clear the user register that was actually connected. (The old
        // code wrote the channel-select value into user register *0* —
        // ASYNCUSER0/SYNCUSER0, i.e. TCB0/TCA0 — silently routing this
        // channel into an unrelated peripheral while the real user stayed
        // connected.)
        self.write_user(User::FILE, User::MULTIPLEXER_INDEX, 0);

        let Channel { evsys, index, state, .. } = self;
        (
            Channel {
                evsys,
                index,
                state: GeneratorAssigned,
                _phantom: PhantomData,
            },
            state.user,
        )
    }
}

impl<Evsys, Flavor, Index> Channel<Evsys, Flavor, Index, GeneratorAssigned>
where
    Evsys: marker::Evsys,
    Flavor: marker::ChannelFlavor,
    Index: marker::Index,
    Self: ChannelConfigurator<Flavor>,
{
    pub fn free_generator(mut self) -> Channel<Evsys, Flavor, Index, Unconfigured> {
        self.set_generator(0);
        self.with_state(Unconfigured)
    }
}

pub trait ChannelConfigurator<F> {
    fn set_generator(&mut self, generator: u8);
}

impl<Evsys, Index, State> ChannelConfigurator<Async> for Channel<Evsys, Async, Index, State>
where
    Evsys: marker::Evsys,
    Index: marker::Index,
    State: marker::ChannelState,
{
    fn set_generator(&mut self, generator: u8) {
        unsafe { (*self.evsys.ptr()).set_async_generator(self.index.index(), generator) }
    }
}

impl<Evsys, Index, State> ChannelConfigurator<Sync> for Channel<Evsys, Sync, Index, State>
where
    Evsys: marker::Evsys,
    Index: marker::Index,
    State: marker::ChannelState,
{
    fn set_generator(&mut self, generator: u8) {
        unsafe { (*self.evsys.ptr()).set_sync_generator(self.index.index(), generator) }
    }
}

/// Event user token for the TCA0 event input (SYNCUSER0)
///
/// Handed out once by [`EvsysExt::split`]; consumed by
/// [`Channel::connect_event_user`] on a sync channel.
pub struct UserTca0 {
    _private: (),
}

/// Event user token for the USART0 IrDA event input (SYNCUSER1)
///
/// Handed out once by [`EvsysExt::split`]; consumed by
/// [`Channel::connect_event_user`] on a sync channel.
pub struct UserUsart0 {
    _private: (),
}

impl crate::private::Sealed for UserTca0 {}
impl crate::private::Sealed for UserUsart0 {}

impl EventUser<Evsys, Sync> for UserTca0 {
    const MULTIPLEXER_INDEX: u8 = 0;
    const FILE: UserRegisterFile = UserRegisterFile::Sync;
}

impl EventUser<Evsys, Sync> for UserUsart0 {
    const MULTIPLEXER_INDEX: u8 = 1;
    const FILE: UserRegisterFile = UserRegisterFile::Sync;
}

// TODO: Tokens for the remaining async-file users (ASYNCUSER0..7: TCB0,
//       ADC0, the four CCL LUT event inputs, TCD0 EV0/EV1). They need a
//       story for who owns the token — the peripheral driver or the EVSYS
//       Parts — and are not required by anything in-tree yet.

pub trait EventGenerator<Evsys, Flavor, Index>
where
    Evsys: marker::Evsys,
    Flavor: marker::ChannelFlavor,
    Index: marker::Index,
{
    type EventSource;

    fn connect_event_generator(
        &mut self,
        channel: Channel<Evsys, Flavor, Index, Unconfigured>,
        source: Self::EventSource,
    ) -> Channel<Evsys, Flavor, Index, GeneratorAssigned>;
}

evsys!({
    channels: [
        {
            channel: 0,
            userindex: 3,
            flavor: Async,
            // generators: {
            //     OFF             => 0x00,
            //     CCL_LUT0        => 0x01,
            //     CCL_LUT1        => 0x02,
            //     AC0_OUT         => 0x03,
            //     TCD0_CMPBCLR    => 0x04,
            //     TCD0_CMPASET    => 0x05,
            //     TCD0_CMPBSET    => 0x06,
            //     TCD0_PROGEV     => 0x07,
            //     RTC_OVF         => 0x08,
            //     RTC_CMP         => 0x09,
            //     PORTA_PIN0      => 0x0A,
            //     PORTA_PIN1      => 0x0B,
            //     PORTA_PIN2      => 0x0C,
            //     PORTA_PIN3      => 0x0D,
            //     PORTA_PIN4      => 0x0E,
            //     PORTA_PIN5      => 0x0F,
            //     PORTA_PIN6      => 0x10,
            //     PORTA_PIN7      => 0x11,
            //     UPDI            => 0x12,
            // }
        },
        {
            channel: 1,
            userindex: 4,
            flavor: Async,
            // generators: {
            //     OFF             => 0,
            //     CCL_LUT0        => 0x01,
            //     CCL_LUT1        => 0x02,
            //     AC0_OUT         => 0x03,
            //     TCD0_CMPBCLR    => 0x04,
            //     TCD0_CMPASET    => 0x05,
            //     TCD0_CMPBSET    => 0x06,
            //     TCD0_PROGEV     => 0x07,
            //     RTC_OVF         => 0x08,
            //     RTC_CMP         => 0x09,
            //     PORTB_PIN0      => 0x0A,
            //     PORTB_PIN1      => 0x0B,
            //     PORTB_PIN2      => 0x0C,
            //     PORTB_PIN3      => 0x0D,
            //     PORTB_PIN4      => 0x0E,
            //     PORTB_PIN5      => 0x0F,
            //     PORTB_PIN6      => 0x10,
            //     PORTB_PIN7      => 0x11,
            // }
        },
        {
            channel: 2,
            userindex: 5,
            flavor: Async,
            // generators: {
            //     OFF             => 0,
            //     CCL_LUT0        => 0x01,
            //     CCL_LUT1        => 0x02,
            //     AC0_OUT         => 0x03,
            //     TCD0_CMPBCLR    => 0x04,
            //     TCD0_CMPASET    => 0x05,
            //     TCD0_CMPBSET    => 0x06,
            //     TCD0_PROGEV     => 0x07,
            //     RTC_OVF         => 0x08,
            //     RTC_CMP         => 0x09,
            //     PORTC_PIN0      => 0x0A,
            //     PORTC_PIN1      => 0x0B,
            //     PORTC_PIN2      => 0x0C,
            //     PORTC_PIN3      => 0x0D,
            //     PORTC_PIN4      => 0x0E,
            //     PORTC_PIN5      => 0x0F,
            // }
        },
        {
            channel: 3,
            userindex: 6,
            flavor: Async,
            // generators: {
            //     OFF             => 0,
            //     CCL_LUT0        => 0x01,
            //     CCL_LUT1        => 0x02,
            //     AC0_OUT         => 0x03,
            //     TCD0_CMPBCLR    => 0x04,
            //     TCD0_CMPASET    => 0x05,
            //     TCD0_CMPBSET    => 0x06,
            //     TCD0_PROGEV     => 0x07,
            //     RTC_OVF         => 0x08,
            //     RTC_CMP         => 0x09,
            //     PIT_DIV8192     => 0x0A,
            //     PIT_DIV4096     => 0x0B,
            //     PIT_DIV2048     => 0x0C,
            //     PIT_DIV1024     => 0x0D,
            //     PIT_DIV512      => 0x0E,
            //     PIT_DIV256      => 0x0F,
            //     PIT_DIV128      => 0x10,
            //     PIT_DIV64       => 0x11,
            // }
        },
        {
            channel: 0,
            userindex: 1,
            flavor: Sync,
            // generators: {
            //     OFF             => 0,
            //     TCB0            => 0x01,
            //     TCA0_OVF_LUNF   => 0x02,
            //     TCA0_HUNF       => 0x03,
            //     TCA0_CMP0       => 0x04,
            //     TCA0_CMP1       => 0x05,
            //     TCA0_CMP2       => 0x06,
            //     PORTC_PIN0      => 0x07,
            //     PORTC_PIN1      => 0x08,
            //     PORTC_PIN2      => 0x09,
            //     PORTC_PIN3      => 0x0A,
            //     PORTC_PIN4      => 0x0B,
            //     PORTC_PIN5      => 0x0C,
            //     PORTA_PIN0      => 0x0D,
            //     PORTA_PIN1      => 0x0E,
            //     PORTA_PIN2      => 0x0F,
            //     PORTA_PIN3      => 0x10,
            //     PORTA_PIN4      => 0x11,
            //     PORTA_PIN5      => 0x12,
            //     PORTA_PIN6      => 0x13,
            //     PORTA_PIN7      => 0x14,
            // }
        },
        {
            channel: 1,
            userindex: 2,
            flavor: Sync,
            // generators: {
            //     OFF             => 0,
            //     TCB0            => 0x01,
            //     TCA0_OVF_LUNF   => 0x02,
            //     TCA0_HUNF       => 0x03,
            //     TCA0_CMP0       => 0x04,
            //     TCA0_CMP1       => 0x05,
            //     TCA0_CMP2       => 0x06,
            //     PORTB_PIN0      => 0x08,
            //     PORTB_PIN1      => 0x09,
            //     PORTB_PIN2      => 0x0A,
            //     PORTB_PIN3      => 0x0B,
            //     PORTB_PIN4      => 0x0C,
            //     PORTB_PIN5      => 0x0D,
            //     PORTB_PIN6      => 0x0E,
            //     PORTB_PIN7      => 0x0F,
            // }
        },
    ],
});
