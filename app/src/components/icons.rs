//! Lucide-style SVG icon components.
//!
//! Stroke-based icons (24×24 viewBox, `currentColor`, 2px stroke, round
//! caps) — render crisply at any size and inherit their color from the
//! surrounding text. Icons are decorative: `aria-hidden` and never carry
//! text alternatives; nearby text always explains the state.

use leptos::prelude::*;

/// Generates one icon component: an `<svg>` shell with the given nodes.
macro_rules! icon {
    ($(#[$doc:meta])* $name:ident, $($nodes:tt)*) => {
        $(#[$doc])*
        #[component]
        pub fn $name(class: &'static str) -> impl IntoView {
            view! {
                <svg
                    class=class
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                >
                    $($nodes)*
                </svg>
            }
        }
    };
}

icon!(
    /// Check-circle — positive verdict (history found).
    IconCheckCircle,
    <circle cx="12" cy="12" r="10" />
    <path d="m9 12 2 2 4-4" />
);

icon!(
    /// X-circle — negative verdict (no history found).
    IconXCircle,
    <circle cx="12" cy="12" r="10" />
    <path d="m15 9-6 6" />
    <path d="m9 9 6 6" />
);

icon!(
    /// Clock — pending state / "check history" action.
    IconClock,
    <circle cx="12" cy="12" r="10" />
    <polyline points="12 6 12 12 16 14" />
);

icon!(
    /// Magnifying glass — search actions.
    IconSearch,
    <circle cx="11" cy="11" r="8" />
    <path d="m21 21-4.3-4.3" />
);

icon!(
    /// Pill — medication search.
    IconPill,
    <path d="m10.5 20.5 10-10a4.95 4.95 0 1 0-7-7l-10 10a4.95 4.95 0 1 0 7 7Z" />
    <path d="m8.5 8.5 7 7" />
);

icon!(
    /// Gear — settings action.
    IconSettings,
    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
    <circle cx="12" cy="12" r="3" />
);

icon!(
    /// Plug — test-connection action.
    IconPlug,
    <path d="M12 22v-5" />
    <path d="M9 8V2" />
    <path d="M15 8V2" />
    <path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z" />
);

icon!(
    /// Save/floppy — persist settings.
    IconSave,
    <path d="M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z" />
    <path d="M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7" />
    <path d="M7 3v4a1 1 0 0 0 1 1h7" />
);

icon!(
    /// X — close/dismiss.
    IconX,
    <path d="M18 6 6 18" />
    <path d="m6 6 12 12" />
);

icon!(
    /// Person — selected patient.
    IconUser,
    <path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" />
    <circle cx="12" cy="7" r="4" />
);

icon!(
    /// Calendar — date-related panel heading.
    IconCalendar,
    <path d="M8 2v4" />
    <path d="M16 2v4" />
    <rect width="18" height="18" x="3" y="4" rx="2" />
    <path d="M3 10h18" />
);
