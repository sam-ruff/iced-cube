//! One module per component, one file per example.
//!
//! Every example exposes `Message`, a `Default` `Example` with
//! `update(&mut self, Message)` and `view(&self) -> Element<Message>`.
//! The same file is shown verbatim as the code sample on the docs site.

pub mod accordion;
pub mod alert;
pub mod badge;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod combobox;
pub mod command;
pub mod context_menu;
pub mod dialog;
pub mod dropdown_menu;
pub mod icon_button;
pub mod input;
pub mod label;
pub mod popover;
pub mod progress;
pub mod radio;
pub mod scroll_area;
pub mod select;
pub mod separator;
pub mod showcase;
pub mod slider;
pub mod spinner;
pub mod stack;
pub mod switch;
pub mod tabs;
pub mod textarea;
pub mod theme;
pub mod toast;
pub mod tooltip;

crate::stories! {
    ButtonVariants => button::variants {
        id: "button/variants", component: "button", title: "Variants",
        description: "Every visual variant, from primary to link.",
        file: "button/variants.rs", height: 160,
    }
    ButtonSizes => button::sizes {
        id: "button/sizes", component: "button", title: "Sizes",
        description: "Small, medium and large, plus the square icon size.",
        file: "button/sizes.rs", height: 160,
    }
    ButtonWithIcon => button::with_icon {
        id: "button/with-icon", component: "button", title: "With icon",
        description: "Lucide icons before or after the label.",
        file: "button/with_icon.rs", height: 160,
    }
    ButtonCounter => button::counter {
        id: "button/counter", component: "button", title: "Enabled and disabled",
        description: "A button without a message renders disabled.",
        file: "button/counter.rs", height: 200,
    }
    IconButtonToolbar => icon_button::toolbar {
        id: "icon-button/toolbar", component: "icon-button", title: "Formatting toolbar",
        description: "Toggles that stay pressed while a format is on, with labels shown as tooltips.",
        file: "icon_button/toolbar.rs", height: 200,
    }
    IconButtonVariants => icon_button::variants {
        id: "icon-button/variants", component: "icon-button", title: "Variants",
        description: "The same variants as a text button.",
        file: "icon_button/variants.rs", height: 160,
    }
    IconButtonSizes => icon_button::sizes {
        id: "icon-button/sizes", component: "icon-button", title: "Sizes",
        description: "Small, medium and large squares.",
        file: "icon_button/sizes.rs", height: 160,
    }
    IconButtonDisabled => icon_button::disabled {
        id: "icon-button/disabled", component: "icon-button", title: "Enabled and disabled",
        description: "Undo and redo turn off when there is nothing to undo or redo. A disabled icon is drawn at half opacity.",
        file: "icon_button/disabled.rs", height: 160,
    }
    InputDefault => input::default {
        id: "input/default", component: "input", title: "Default",
        description: "A controlled input that submits on Enter.",
        file: "input/default.rs", height: 200,
    }
    InputWithIcon => input::with_icon {
        id: "input/with-icon", component: "input", title: "With icon",
        description: "A leading Lucide icon, in small, medium and large sizes.",
        file: "input/with_icon.rs",
    }
    InputPassword => input::password {
        id: "input/password", component: "input", title: "Password",
        description: "A secure input with a button that reveals the value.",
        file: "input/password.rs", height: 160,
    }
    InputStates => input::states {
        id: "input/states", component: "input", title: "Invalid and disabled",
        description: "An invalid value draws a destructive border. An input without on_input is disabled.",
        file: "input/states.rs", height: 200,
    }
    TextareaDefault => textarea::default {
        id: "textarea/default", component: "textarea", title: "Default",
        description: "Grows with its content, with a character count below.",
        file: "textarea/default.rs",
    }
    TextareaDisabled => textarea::disabled {
        id: "textarea/disabled", component: "textarea", title: "Fixed height and disabled",
        description: "A fixed-height textarea next to one without on_action, which is disabled.",
        file: "textarea/disabled.rs",
    }
    LabelField => label::field {
        id: "label/field", component: "label", title: "Field",
        description: "A label above a control, with a required marker or a description.",
        file: "label/field.rs",
    }
    LabelValidation => label::validation {
        id: "label/validation", component: "label", title: "Validation",
        description: "An error message below the control turns the label destructive.",
        file: "label/validation.rs",
    }
    SeparatorOrientation => separator::orientation {
        id: "separator/orientation", component: "separator", title: "Horizontal and vertical",
        description: "A horizontal rule between sections and vertical rules between links.",
        file: "separator/orientation.rs",
    }
    SeparatorWithLabel => separator::with_label {
        id: "separator/with-label", component: "separator", title: "With label",
        description: "Inline text such as \"or\" between two alternatives.",
        file: "separator/with_label.rs",
    }
    CheckboxDefault => checkbox::default {
        id: "checkbox/default", component: "checkbox", title: "Default",
        description: "A labelled checkbox bound to a bool.",
        file: "checkbox/default.rs", height: 160,
    }
    CheckboxStates => checkbox::states {
        id: "checkbox/states", component: "checkbox", title: "States",
        description: "Unchecked, checked and indeterminate, enabled and disabled.",
        file: "checkbox/states.rs", height: 200,
    }
    CheckboxSelectAll => checkbox::select_all {
        id: "checkbox/select-all", component: "checkbox", title: "Select all",
        description: "A parent checkbox that turns indeterminate for a mixed selection.",
        file: "checkbox/select_all.rs",
    }
    RadioDefault => radio::default {
        id: "radio/default", component: "radio", title: "Default",
        description: "A vertical group built from an enum's options.",
        file: "radio/default.rs",
    }
    RadioHorizontal => radio::horizontal {
        id: "radio/horizontal", component: "radio", title: "Horizontal",
        description: "The same group laid out in a row.",
        file: "radio/horizontal.rs", height: 160,
    }
    RadioDisabled => radio::disabled {
        id: "radio/disabled", component: "radio", title: "Single radios",
        description: "Individual radios, one of them disabled.",
        file: "radio/disabled.rs", height: 200,
    }
    SwitchDefault => switch::default {
        id: "switch/default", component: "switch", title: "Default",
        description: "A labelled switch bound to a bool.",
        file: "switch/default.rs", height: 160,
    }
    SwitchStates => switch::states {
        id: "switch/states", component: "switch", title: "States",
        description: "On and off, enabled and disabled.",
        file: "switch/states.rs", height: 200,
    }
    SwitchSettings => switch::settings {
        id: "switch/settings", component: "switch", title: "Settings list",
        description: "Switches in a settings list, one depending on another.",
        file: "switch/settings.rs",
    }
    SelectDefault => select::default {
        id: "select/default", component: "select", title: "Default",
        description: "A select with a placeholder until a value is chosen.",
        file: "select/default.rs",
    }
    SelectStates => select::states {
        id: "select/states", component: "select", title: "States",
        description: "Enum options, with enabled and disabled selects.",
        file: "select/states.rs",
    }
    ComboboxDefault => combobox::default {
        id: "combobox/default", component: "combobox", title: "Default",
        description: "Suggestions filter as you type. The list starts open here so you can see it.",
        file: "combobox/default.rs", height: 400,
    }
    ComboboxEmpty => combobox::empty {
        id: "combobox/empty", component: "combobox", title: "Empty results",
        description: "A message in the list when nothing matches the query.",
        file: "combobox/empty.rs", height: 200,
    }
    ComboboxField => combobox::field {
        id: "combobox/field", component: "combobox", title: "In a field",
        description: "A labelled combobox showing its selected value, above a disabled one.",
        file: "combobox/field.rs", height: 360,
    }
    ComboboxKeyboard => combobox::keyboard {
        id: "combobox/keyboard", component: "combobox", title: "Keyboard shortcuts",
        description: "The default keymap with Ctrl+N and Ctrl+P added. The combobox handles its keys itself while it has focus.",
        file: "combobox/keyboard.rs", height: 420,
    }
    SliderDefault => slider::default {
        id: "slider/default", component: "slider", title: "Default",
        description: "A labelled slider that shows its value.",
        file: "slider/default.rs", height: 160,
    }
    SliderSteps => slider::steps {
        id: "slider/steps", component: "slider", title: "Steps",
        description: "Snapping to a step, with a custom value format.",
        file: "slider/steps.rs",
    }
    SliderKeyboard => slider::keyboard {
        id: "slider/keyboard", component: "slider", title: "Keyboard shortcuts",
        description: "Arrow keys, Home and End through the default keymap, plus an added binding.",
        file: "slider/keyboard.rs", height: 200, subscription: true,
    }
    SliderDisabled => slider::disabled {
        id: "slider/disabled", component: "slider", title: "Disabled",
        description: "A slider without a message renders disabled.",
        file: "slider/disabled.rs",
    }
    BadgeVariants => badge::variants {
        id: "badge/variants", component: "badge", title: "Variants",
        description: "Every badge variant, from default to warning.",
        file: "badge/variants.rs", height: 160,
    }
    BadgeWithIcon => badge::with_icon {
        id: "badge/with-icon", component: "badge", title: "With icon",
        description: "A small Lucide icon before the label.",
        file: "badge/with_icon.rs", height: 160,
    }
    BadgeInAList => badge::in_a_list {
        id: "badge/in-a-list", component: "badge", title: "In a list",
        description: "Status badges aligned against list rows.",
        file: "badge/in_a_list.rs",
    }
    CardBasic => card::basic {
        id: "card/basic", component: "card", title: "Header, body and footer",
        description: "A card with a title, description, body text and footer actions.",
        file: "card/basic.rs",
    }
    CardStats => card::stats {
        id: "card/stats", component: "card", title: "Stat cards",
        description: "Cards sharing a row, each with a figure and a badge.",
        file: "card/stats.rs",
    }
    ScrollAreaVertical => scroll_area::vertical {
        id: "scroll-area/vertical", component: "scroll-area", title: "Vertical",
        description: "A long list inside a fixed height.",
        file: "scroll_area/vertical.rs",
    }
    ScrollAreaHorizontal => scroll_area::horizontal {
        id: "scroll-area/horizontal", component: "scroll-area", title: "Horizontal",
        description: "A row of cards wider than its container.",
        file: "scroll_area/horizontal.rs",
    }
    ScrollAreaBoth => scroll_area::both {
        id: "scroll-area/both", component: "scroll-area", title: "Both directions",
        description: "A grid that overflows both ways.",
        file: "scroll_area/both.rs",
    }
    StackGaps => stack::gaps {
        id: "stack/gaps", component: "stack", title: "Gaps",
        description: "Each step of the spacing scale, from Xs to Xl.",
        file: "stack/gaps.rs",
    }
    StackAlignment => stack::alignment {
        id: "stack/alignment", component: "stack", title: "Alignment",
        description: "Children aligned to the start, centre or end.",
        file: "stack/alignment.rs",
    }
    StackWrap => stack::wrap {
        id: "stack/wrap", component: "stack", title: "Wrapping",
        description: "A horizontal stack that flows onto new lines.",
        file: "stack/wrap.rs",
    }
    AlertVariants => alert::variants {
        id: "alert/variants", component: "alert", title: "Variants",
        description: "Info, success, warning and destructive callouts.",
        file: "alert/variants.rs",
    }
    AlertCustomIcon => alert::custom_icon {
        id: "alert/custom-icon", component: "alert", title: "Custom icon",
        description: "Any Lucide icon in place of the variant's default.",
        file: "alert/custom_icon.rs",
    }
    ProgressVariants => progress::variants {
        id: "progress/variants", component: "progress", title: "Variants",
        description: "Labelled bars with a percentage, in every colour.",
        file: "progress/variants.rs",
    }
    ProgressSizes => progress::sizes {
        id: "progress/sizes", component: "progress", title: "Sizes",
        description: "Small, medium and large bars.",
        file: "progress/sizes.rs",
    }
    ProgressInteractive => progress::interactive {
        id: "progress/interactive", component: "progress", title: "Interactive",
        description: "The bar follows a value held in app state.",
        file: "progress/interactive.rs", height: 200,
    }
    SpinnerLoading => spinner::loading {
        id: "spinner/loading", component: "spinner", title: "Loading",
        description: "Animates only while loading, driven by window frames.",
        file: "spinner/loading.rs", height: 200, subscription: true,
    }
    SpinnerSizes => spinner::sizes {
        id: "spinner/sizes", component: "spinner", title: "Sizes",
        description: "Small, medium and large spinners sharing one phase.",
        file: "spinner/sizes.rs", height: 160, subscription: true,
    }
    TooltipPositions => tooltip::positions {
        id: "tooltip/positions", component: "tooltip", title: "Positions",
        description: "The bubble on each side of its element.",
        file: "tooltip/positions.rs", height: 180,
    }
    TooltipToolbar => tooltip::toolbar {
        id: "tooltip/toolbar", component: "tooltip", title: "Icon toolbar",
        description: "Labels for icon-only buttons.",
        file: "tooltip/toolbar.rs", height: 180,
    }
    TabsUnderline => tabs::underline {
        id: "tabs/underline", component: "tabs", title: "Underline",
        description: "Labels over a rule, with a disabled tab.",
        file: "tabs/underline.rs", height: 200,
    }
    TabsPills => tabs::pills {
        id: "tabs/pills", component: "tabs", title: "Pills",
        description: "A segmented control with icons.",
        file: "tabs/pills.rs", height: 200,
    }
    TabsKeyboard => tabs::keyboard {
        id: "tabs/keyboard", component: "tabs", title: "Keyboard shortcuts",
        description: "Key presses resolve through the default keymap, skipping disabled tabs. In the browser, use the arrow keys, Home and End: browsers keep Ctrl+Tab for themselves.",
        file: "tabs/keyboard.rs", height: 200, subscription: true,
    }
    CommandInline => command::inline {
        id: "command/inline", component: "command", title: "Inline",
        description: "Grouped actions with icons, shortcut hints, keywords and a disabled row.",
        file: "command/inline.rs", height: 400,
    }
    CommandKeyboard => command::keyboard {
        id: "command/keyboard", component: "command", title: "Keyboard shortcuts",
        description: "The default keymap with Ctrl+N and Ctrl+P added. The list handles keys while its field has focus; the app routes the rest through keys::subscription.",
        file: "command/keyboard.rs", height: 400, subscription: true,
    }
    CommandAsyncSearch => command::async_search {
        id: "command/async-search", component: "command", title: "Async results",
        description: "A background task streams file matches through the channel, tagged with their query. It waits with futures-timer, so it also runs in the browser.",
        file: "command/async_search.rs", height: 380, subscription: true,
    }
    CommandPalette => command::palette {
        id: "command/palette", component: "command", title: "Command palette",
        description: "The list in a dialog, opened by a button or Ctrl+K. Escape clears the query, then closes the palette; Ctrl+K passes through the dialog to close it too.",
        file: "command/palette.rs", height: 460, subscription: true, edge: true,
    }
    AccordionSingle => accordion::single {
        id: "accordion/single", component: "accordion", title: "Single",
        description: "Opening a section closes the others.",
        file: "accordion/single.rs",
    }
    AccordionMultiple => accordion::multiple {
        id: "accordion/multiple", component: "accordion", title: "Multiple",
        description: "Sections open and close independently.",
        file: "accordion/multiple.rs",
    }
    ToastBackgroundJob => toast::background_job {
        id: "toast/background-job", component: "toast", title: "Background job",
        description: "A producer sends toasts through the channel. It waits with futures-timer rather than a thread, so it also runs in the browser.",
        file: "toast/background_job.rs", height: 340, subscription: true,
    }
    ToastVariants => toast::variants {
        id: "toast/variants", component: "toast", title: "Variants",
        description: "Default, success and destructive toasts.",
        file: "toast/variants.rs", height: 340, subscription: true,
    }
    ToastAction => toast::action {
        id: "toast/action", component: "toast", title: "With action",
        description: "An undo action in the top right corner.",
        file: "toast/action.rs", height: 340, subscription: true,
    }
    ToastKeyboard => toast::keyboard {
        id: "toast/keyboard", component: "toast", title: "Keyboard shortcuts",
        description: "Escape closes the newest toast and Shift+Escape closes them all, through the default keymap.",
        file: "toast/keyboard.rs", height: 340, subscription: true,
    }
    DialogDefault => dialog::default {
        id: "dialog/default", component: "dialog", title: "Default",
        description: "A trigger button opens the dialog. Escape, the close button or a click on the scrim dismisses it.",
        file: "dialog/default.rs", height: 320, edge: true,
    }
    DialogForm => dialog::form {
        id: "dialog/form", component: "dialog", title: "Form and keyboard",
        description: "Fields inside a small dialog. Tab and Shift+Tab move focus between them through the default keymap, without leaving the dialog.",
        file: "dialog/form.rs", height: 420, edge: true,
    }
    DialogDestructive => dialog::destructive {
        id: "dialog/destructive", component: "dialog", title: "Destructive confirmation",
        description: "An alert dialog with Cancel and a destructive action. Clicking the scrim does nothing.",
        file: "dialog/destructive.rs", height: 320, edge: true,
    }
    PopoverSettings => popover::settings {
        id: "popover/settings", component: "popover", title: "Settings form",
        description: "A small form in a popover that closes on Escape or a click outside.",
        file: "popover/settings.rs", height: 360,
    }
    PopoverPlacement => popover::placement {
        id: "popover/placement", component: "popover", title: "Side and alignment",
        description: "A popover on each side of its trigger, aligned to the start, centre or end.",
        file: "popover/placement.rs", height: 280,
    }
    PopoverKeyboard => popover::keyboard {
        id: "popover/keyboard", component: "popover", title: "Keyboard shortcuts",
        description: "Escape closes the popover, and an added binding toggles it with I.",
        file: "popover/keyboard.rs", height: 280, subscription: true,
    }
    DropdownMenuDefault => dropdown_menu::default {
        id: "dropdown-menu/default", component: "dropdown-menu", title: "Default",
        description: "Items with icons and shortcut hints, a submenu, a disabled item and a destructive one.",
        file: "dropdown_menu/default.rs", height: 460,
    }
    DropdownMenuCheckboxes => dropdown_menu::checkboxes {
        id: "dropdown-menu/checkboxes", component: "dropdown-menu", title: "Checkbox items",
        description: "Items that toggle a check mark, one of them disabled.",
        file: "dropdown_menu/checkboxes.rs", height: 280,
    }
    DropdownMenuRadioItems => dropdown_menu::radio_items {
        id: "dropdown-menu/radio-items", component: "dropdown-menu", title: "Radio items",
        description: "A group of items where choosing one unchecks the others.",
        file: "dropdown_menu/radio_items.rs", height: 280,
    }
    DropdownMenuSubmenu => dropdown_menu::submenu {
        id: "dropdown-menu/submenu", component: "dropdown-menu", title: "Submenu",
        description: "A nested menu that opens beside its item, on hover or with the right arrow key.",
        file: "dropdown_menu/submenu.rs", height: 340,
    }
    DropdownMenuKeyboard => dropdown_menu::keyboard {
        id: "dropdown-menu/keyboard", component: "dropdown-menu", title: "Keyboard shortcuts",
        description: "The open menu resolves its own keys through the keymap, with typeahead for letters. The app binds Alt+ArrowDown to open it.",
        file: "dropdown_menu/keyboard.rs", height: 400, subscription: true,
    }
    DropdownMenuInDialog => dropdown_menu::in_dialog {
        id: "dropdown-menu/in-dialog", component: "dropdown-menu", title: "In a dialog",
        description: "A menu inside a dialog. A click outside or Escape closes only the menu; the next one closes the dialog.",
        file: "dropdown_menu/in_dialog.rs", height: 400, edge: true,
    }
    ContextMenuDefault => context_menu::default {
        id: "context-menu/default", component: "context-menu", title: "Default",
        description: "A right-click menu with shortcut hints, a submenu and checkbox items.",
        file: "context_menu/default.rs", height: 300,
    }
    ContextMenuKeyboard => context_menu::keyboard {
        id: "context-menu/keyboard", component: "context-menu", title: "Keyboard shortcuts",
        description: "Shift+F10 or the Menu key opens the menu at the corner of the area, with radio items to sort by.",
        file: "context_menu/keyboard.rs", height: 400, subscription: true,
    }
    ThemeCustom => theme::custom {
        id: "theme/custom", component: "theme", title: "Custom palette",
        description: "A brand palette built with theme::Config. The switch rebuilds it from the dark defaults.",
        file: "theme/custom.rs", height: 240, theme: true,
    }
    ThemePalettes => theme::palettes {
        id: "theme/palettes", component: "theme", title: "Light, dark and custom",
        description: "The same controls under the light theme, the dark theme and a Config palette, side by side.",
        file: "theme/palettes.rs", height: 280,
    }
    ThemeTokens => theme::tokens {
        id: "theme/tokens", component: "theme", title: "Tokens",
        description: "Every semantic colour token, as resolved from the current theme.",
        file: "theme/tokens.rs", height: 260,
    }
    ShowcaseSettings => showcase::settings {
        id: "showcase/settings", component: "showcase", title: "Preferences",
        description: "Switches, a select, a slider, a badge and buttons working together in a card.",
        file: "showcase/settings.rs", height: 400,
    }
}
