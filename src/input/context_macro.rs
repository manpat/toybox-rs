
#[macro_export]
macro_rules! declare_input_context {
	(
		struct $context_ident:ident $context_name:tt {
			$( $body:tt )*
		}
	) => {
		$crate::declare_input_context!( @build { $($body)* } $context_ident, $context_name, [], );
	};



	( @build
		{
			priority [$priority:expr]
			$($rest:tt)*
		}
		$context_ident:ident, $context_name:tt, [$( $_priority:expr )?], $($bindings:tt)* )
	=> {
		$crate::declare_input_context!(@build {$($rest)*} $context_ident, $context_name, [ $priority ], $( $bindings )*);
	};



	( @build
		{
			$binding_type:ident $binding_ident:ident { $( $binding_name_and_default:tt )+ }
			$($rest:tt)*
		}
		$context_ident:ident, $context_name:tt, [$( $priority:tt )*], $($bindings:tt)* )
	=> {
		$crate::declare_input_context!(@build {$($rest)*} $context_ident, $context_name, [ $($priority)* ],
			$( $bindings )*
			$binding_ident { $crate::__input__new_action!($binding_type, $($binding_name_and_default)+) }
		);
	};



	( @build {}
		$context_ident:ident, $context_name:tt, [$( $priority:expr )?],
		$( $binding_ident:ident { $action_expr:expr } )*
	) => {
		#[derive(Copy, Clone, Debug)]
		pub struct $context_ident {
			__context_id: $crate::input::ContextID,

			$(
				pub $binding_ident: $crate::input::ActionID,
			)*
		}

		impl $context_ident {
			pub fn new(system: &mut $crate::input::InputSystem) -> Self {
				let mut __ctx = system.new_context($context_name);
				$( __ctx.set_priority($priority); )?

				$(
					let $binding_ident = __ctx.new_action($action_expr);
				)*

				Self {
					__context_id: __ctx.build(),
					$( $binding_ident, )*
				}
			}

			pub fn new_active(system: &mut $crate::input::InputSystem) -> Self {
				let __ctx = Self::new(system);
				system.enter_context(__ctx.context_id());
				__ctx
			}

			pub fn context_id(&self) -> $crate::input::ContextID { self.__context_id }
		}
	};
}



#[macro_export]
#[doc(hidden)]
macro_rules! __input__new_action {
	(trigger, $name:tt [$default:expr]) => { $crate::input::Action::new_trigger($name, $default) };
	(state, $name:tt [$default:expr]) => { $crate::input::Action::new_state($name, $default) };
	(mouse, $name:tt [$default:expr]) => { $crate::input::Action::new_mouse($name, $crate::input::MouseSpace::LegacyPixelRatio, $default) };
	(pointer, $name:tt) => { $crate::input::Action::new_pointer($name, $crate::input::MouseSpace::PreserveAspect) };
}

