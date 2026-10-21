//! One module per component, one file per example.
//!
//! Every example exposes `Message`, a `Default` `Example` with
//! `update(&mut self, Message)` and `view(&self) -> Element<Message>`.
//! The same file is shown verbatim as the code sample on the docs site.

pub mod accordion;
pub mod alert;
pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod combobox;
pub mod command;
pub mod command_palette;
pub mod context_menu;
pub mod data_table;
pub mod dialog;
pub mod document_tabs;
pub mod dropdown_menu;
pub mod icon_button;
pub mod input;
pub mod json_viewer;
pub mod label;
pub mod log_viewer;
pub mod menubar;
pub mod pagination;
pub mod popover;
pub mod progress;
pub mod property_grid;
pub mod radio;
pub mod resizable_panel;
pub mod scroll_area;
pub mod select;
pub mod separator;
pub mod showcase;
pub mod sidebar;
pub mod slider;
pub mod spinner;
pub mod split_pane;
pub mod stack;
pub mod status_bar;
pub mod switch;
pub mod tabs;
pub mod textarea;
pub mod theme;
pub mod toast;
pub mod toggle;
pub mod toggle_group;
pub mod toolbar;
pub mod tooltip;
pub mod tree;
pub mod virtual_list;

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
    ToggleDefault => toggle::default {
        id: "toggle/default", component: "toggle", title: "Default",
        description: "Toggles with icons that stay pressed while an option is on.",
        file: "toggle/default.rs", height: 160,
    }
    ToggleVariants => toggle::variants {
        id: "toggle/variants", component: "toggle", title: "Variants",
        description: "Default and outline toggles, each next to a disabled toggle that is on.",
        file: "toggle/variants.rs", height: 200,
    }
    ToggleKeyboard => toggle::keyboard {
        id: "toggle/keyboard", component: "toggle", title: "Keyboard shortcuts",
        description: "Each toggle's keymap swaps the default Space for its own chord.",
        file: "toggle/keyboard.rs", height: 160, subscription: true,
    }
    ToggleGroupSingle => toggle_group::single {
        id: "toggle-group/single", component: "toggle-group", title: "Single choice",
        description: "An outlined group where one alignment is always on.",
        file: "toggle_group/single.rs", height: 160,
    }
    ToggleGroupMultiple => toggle_group::multiple {
        id: "toggle-group/multiple", component: "toggle-group", title: "Multiple choice",
        description: "Formatting toggles that switch on and off independently, with a disabled item.",
        file: "toggle_group/multiple.rs", height: 160,
    }
    ToggleGroupKeyboard => toggle_group::keyboard {
        id: "toggle-group/keyboard", component: "toggle-group", title: "Keyboard shortcuts",
        description: "The arrow keys move a single-choice group's selection through the default keymap.",
        file: "toggle_group/keyboard.rs", height: 160, subscription: true,
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
        description: "A labelled combobox showing its selected value, a required one marked invalid until something is picked, and a disabled one.",
        file: "combobox/field.rs", height: 440,
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
    AvatarFallbacks => avatar::fallbacks {
        id: "avatar/fallbacks", component: "avatar", title: "Image, initials and icon",
        description: "An image when there is one, otherwise the name's initials, an icon, or a person icon when there is no name.",
        file: "avatar/fallbacks.rs", height: 160,
    }
    AvatarSizes => avatar::sizes {
        id: "avatar/sizes", component: "avatar", title: "Sizes and shapes",
        description: "Small to extra large, as circles for people and rounded squares for teams.",
        file: "avatar/sizes.rs", height: 200,
    }
    AvatarPresence => avatar::presence {
        id: "avatar/presence", component: "avatar", title: "Presence",
        description: "A dot on the corner for online, away, busy and offline.",
        file: "avatar/presence.rs",
    }
    AvatarGroup => avatar::group {
        id: "avatar/group", component: "avatar", title: "Group",
        description: "Overlapping avatars, with a count for the ones that do not fit.",
        file: "avatar/group.rs", height: 160,
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
        file: "scroll_area/both.rs", height: 300,
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
    BreadcrumbDefault => breadcrumb::default {
        id: "breadcrumb/default", component: "breadcrumb", title: "Default",
        description: "A folder path with a home icon. Clicking a level opens it and trims the trail.",
        file: "breadcrumb/default.rs", height: 160,
    }
    BreadcrumbCollapsed => breadcrumb::collapsed {
        id: "breadcrumb/collapsed", component: "breadcrumb", title: "Collapsed",
        description: "A long trail with slash separators keeps its first and last levels. The ellipsis shows the rest.",
        file: "breadcrumb/collapsed.rs", height: 160,
    }
    BreadcrumbKeyboard => breadcrumb::keyboard {
        id: "breadcrumb/keyboard", component: "breadcrumb", title: "Keyboard shortcuts",
        description: "Alt+Up goes to the parent level and Alt+Shift+Up to the root, through the default keymap.",
        file: "breadcrumb/keyboard.rs", height: 160, subscription: true,
    }
    PaginationNumbers => pagination::numbers {
        id: "pagination/numbers", component: "pagination", title: "Page numbers",
        description: "The first, last and nearby pages, with gaps for the rest. On a narrow screen it drops pages to fit.",
        file: "pagination/numbers.rs", height: 160,
    }
    PaginationCompact => pagination::compact {
        id: "pagination/compact", component: "pagination", title: "Compact",
        description: "A position label between first, previous, next and last buttons, paging a short list.",
        file: "pagination/compact.rs", height: 260,
    }
    PaginationKeyboard => pagination::keyboard {
        id: "pagination/keyboard", component: "pagination", title: "Keyboard shortcuts",
        description: "Page Up, Page Down, Ctrl+Home and Ctrl+End through the default keymap.",
        file: "pagination/keyboard.rs", height: 200, subscription: true,
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
    SidebarDefault => sidebar::default {
        id: "sidebar/default", component: "sidebar", title: "Default",
        description: "A workspace switcher, groups with icons and a badge, a parent with nested items, a collapsible group with a disabled item, and a user menu in the footer. On a phone it becomes a drawer.",
        file: "sidebar/default.rs", height: 460, edge: true,
    }
    SidebarRail => sidebar::rail {
        id: "sidebar/rail", component: "sidebar", title: "Collapsed to a rail",
        description: "Icons only, with the label in a tooltip. Ctrl+B toggles the rail through the default keymap, routed from keys::subscription with State::shortcut.",
        file: "sidebar/rail.rs", height: 360, subscription: true, edge: true,
    }
    SidebarDrawer => sidebar::drawer {
        id: "sidebar/drawer", component: "sidebar", title: "Drawer on narrow windows",
        description: "Below the breakpoint the sidebar slides over the page on a scrim, opened by the trigger in the bar above the content. This example sets the breakpoint so high that it always shows the drawer.",
        file: "sidebar/drawer.rs", height: 400, edge: true,
    }
    SidebarResizable => sidebar::resizable {
        id: "sidebar/resizable", component: "sidebar", title: "In a resizable panel",
        description: "The sidebar fills a collapsible panel beside the page. Dragging the panel below its minimum snaps it to the width of the rail.",
        file: "sidebar/resizable.rs", height: 360, edge: true,
    }
    DocumentTabsEditor => document_tabs::editor {
        id: "document-tabs/editor", component: "document-tabs", title: "Editor",
        description: "Open files with icons, a pinned README, unsaved changes shown as dots and a new tab button. Drag a tab to move it, middle-click to close it, or right-click for more, including an app item that copies the path.",
        file: "document_tabs/editor.rs", height: 320,
    }
    DocumentTabsOverflow => document_tabs::overflow {
        id: "document-tabs/overflow", component: "document-tabs", title: "Many tabs",
        description: "Tabs that do not fit scroll sideways, keeping the active one in view, and a button at the end lists them all.",
        file: "document_tabs/overflow.rs", height: 200,
    }
    DocumentTabsConfirmClose => document_tabs::confirm_close {
        id: "document-tabs/confirm-close", component: "document-tabs", title: "Confirm before closing",
        description: "Closing only asks. The app checks for unsaved changes, confirms with an alert dialog, then closes the tabs itself.",
        file: "document_tabs/confirm_close.rs", height: 360, edge: true,
    }
    DocumentTabsKeyboard => document_tabs::keyboard {
        id: "document-tabs/keyboard", component: "document-tabs", title: "Keyboard shortcuts",
        description: "The default keymap with Alt chords added, since browsers keep Ctrl+Tab, Ctrl+W and Ctrl+PageDown for themselves. Releasing every modifier ends a walk through recent tabs.",
        file: "document_tabs/keyboard.rs", height: 280, subscription: true,
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
    ResizablePanelHorizontal => resizable_panel::horizontal {
        id: "resizable-panel/horizontal", component: "resizable-panel", title: "Horizontal",
        description: "Three panels in a row with grips on the handles. Each shows its share of the width.",
        file: "resizable_panel/horizontal.rs", height: 240,
    }
    ResizablePanelVertical => resizable_panel::vertical {
        id: "resizable-panel/vertical", component: "resizable-panel", title: "Vertical",
        description: "A header and content stacked in a column.",
        file: "resizable_panel/vertical.rs",
    }
    ResizablePanelNested => resizable_panel::nested {
        id: "resizable-panel/nested", component: "resizable-panel", title: "Nested",
        description: "A column of two panels inside a row. Below 480 pixels the row stacks into a column.",
        file: "resizable_panel/nested.rs",
    }
    ResizablePanelCollapsible => resizable_panel::collapsible {
        id: "resizable-panel/collapsible", component: "resizable-panel", title: "Collapsible with limits",
        description: "A folder list between 140 pixels and half the width that snaps shut below its threshold, with a button that toggles it.",
        file: "resizable_panel/collapsible.rs", height: 260,
    }
    ResizablePanelKeyboard => resizable_panel::keyboard {
        id: "resizable-panel/keyboard", component: "resizable-panel", title: "Keyboard shortcuts",
        description: "A focused handle resolves the default keymap, with Page Up and Page Down added. A press focuses the handle, and so does widget::operation::focus with its id.",
        file: "resizable_panel/keyboard.rs", height: 280, task: true,
    }
    SplitPaneEditorPreview => split_pane::editor_preview {
        id: "split-pane/editor-preview", component: "split-pane", title: "Editor and preview",
        description: "A text editor beside its live preview. Below 480 pixels the two stack.",
        file: "split_pane/editor_preview.rs", height: 340,
    }
    SplitPaneVertical => split_pane::vertical {
        id: "split-pane/vertical", component: "split-pane", title: "Vertical",
        description: "A query above its results, each with a minimum height.",
        file: "split_pane/vertical.rs", height: 340,
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
        description: "An undo action in the top right corner that hands back the message which reverses the archive. It stays at least ten seconds, pauses under the pointer, and Alt+Z presses it.",
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
        description: "Fields inside a small dialog. Tab and Shift+Tab move focus between them through the default keymap, without leaving the dialog, and Enter saves through on_confirm.",
        file: "dialog/form.rs", height: 420, edge: true,
    }
    DialogDestructive => dialog::destructive {
        id: "dialog/destructive", component: "dialog", title: "Destructive confirmation",
        description: "An alert dialog with Cancel and a destructive action. Enter confirms, Escape cancels and clicking the scrim does nothing.",
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
        description: "A menu opened by a right-click or a long press, with shortcut hints, a submenu and checkbox items.",
        file: "context_menu/default.rs", height: 300,
    }
    ContextMenuKeyboard => context_menu::keyboard {
        id: "context-menu/keyboard", component: "context-menu", title: "One menu for every row",
        description: "A file list shares one menu. Up and Down pick a file, and Shift+F10 or the Menu key opens its menu below it.",
        file: "context_menu/keyboard.rs", height: 400, subscription: true,
    }
    MenubarEditor => menubar::editor {
        id: "menubar/editor", component: "menubar", title: "Editor menus",
        description: "File, Edit, View and Help with shortcut hints, a submenu, checkbox and radio items. Once a menu is open, moving onto another title switches to it. F10 or Alt focuses the bar and the arrows move through it.",
        file: "menubar/editor.rs", height: 400, subscription: true, edge: true,
    }
    ToolbarFormatting => toolbar::formatting {
        id: "toolbar/formatting", component: "toolbar", title: "Formatting toolbar",
        description: "Toggles, a single-choice alignment group, undo and redo. Drag the slider: whatever does not fit moves into the More menu. Ctrl+F10 focuses the toolbar for the arrow keys.",
        file: "toolbar/formatting.rs", height: 360, subscription: true,
    }
    StatusBarBackground => status_bar::background {
        id: "status-bar/background", component: "status-bar", title: "Background updates",
        description: "A simulated worker sends git sync, build progress and cursor updates through the channel. It waits with futures-timer, so it also runs in the browser.",
        file: "status_bar/background.rs", height: 200, subscription: true, edge: true,
    }
    CommandPalettePages => command_palette::pages {
        id: "command-palette/pages", component: "command-palette", title: "Pages and recent commands",
        description: "Ctrl+K or Ctrl+Shift+P opens it anywhere. Recent commands come first, and Change theme opens a page of its own; Backspace on an empty query goes back.",
        file: "command_palette/pages.rs", height: 460, subscription: true, edge: true,
    }
    CommandPaletteAsyncSearch => command_palette::async_search {
        id: "command-palette/async-search", component: "command-palette", title: "Async results",
        description: "Typing asks the app to search, and a background task streams file matches through the command list's channel.",
        file: "command_palette/async_search.rs", height: 460, subscription: true, edge: true,
    }
    TreeFileExplorer => tree::file_explorer {
        id: "tree/file-explorer", component: "tree", title: "File explorer",
        description: "Folders that open and close, file sizes and a count badge, indentation guides, a disabled folder, a name too long to fit and a context menu on every node.",
        file: "tree/file_explorer.rs", height: 380,
    }
    TreeCheckbox => tree::checkbox {
        id: "tree/checkbox", component: "tree", title: "Checkboxes",
        description: "Checking a group checks its members, and a group with some members checked shows a dash. The disabled permission keeps its state.",
        file: "tree/checkbox.rs", height: 440,
    }
    TreeLazyLoading => tree::lazy_loading {
        id: "tree/lazy-loading", component: "tree", title: "Lazy loading",
        description: "Folders fetch their children when first opened. A simulated server sends them through the tree's channel after a short wait, using futures-timer so it also runs in the browser.",
        file: "tree/lazy_loading.rs", height: 360, subscription: true,
    }
    TreeKeyboard => tree::keyboard {
        id: "tree/keyboard", component: "tree", title: "Keyboard shortcuts",
        description: "A multiple selection driven by the default keymap with J and K added. The tree resolves keys only while it has focus.",
        file: "tree/keyboard.rs", height: 400,
    }
    DataTableDefault => data_table::default {
        id: "data-table/default", component: "data-table", title: "Payments",
        description: "Sortable headers, a search box, a status filter, a Columns menu, status badges and right-aligned amounts.",
        file: "data_table/default.rs", height: 460,
    }
    DataTablePagination => data_table::pagination {
        id: "data-table/pagination", component: "data-table", title: "Pagination",
        description: "243 invoices in pages of 10, 25, 50 or 100. The rows scroll under a header that stays put, and only the rows in view are built.",
        file: "data_table/pagination.rs", height: 480,
    }
    DataTableSelection => data_table::selection {
        id: "data-table/selection", component: "data-table", title: "Selection and bulk actions",
        description: "A checkbox on every row, and a header checkbox that selects or clears the rows on the page. Buttons above act on the selection.",
        file: "data_table/selection.rs", height: 440,
    }
    DataTableRowActions => data_table::row_actions {
        id: "data-table/row-actions", component: "data-table", title: "Row actions",
        description: "One context menu shared by every row, opened by a right-click, a long press, the row's menu button or Shift+F10. The progress column is a custom cell.",
        file: "data_table/row_actions.rs", height: 360,
    }
    DataTableLoading => data_table::loading {
        id: "data-table/loading", component: "data-table", title: "Loading",
        description: "Placeholder rows while the data loads. The switch toggles them.",
        file: "data_table/loading.rs", height: 440,
    }
    DataTableEmpty => data_table::empty {
        id: "data-table/empty", component: "data-table", title: "Empty",
        description: "A message in place of rows when nothing matches, with a button that clears the search.",
        file: "data_table/empty.rs", height: 340,
    }
    DataTableKeyboard => data_table::keyboard {
        id: "data-table/keyboard", component: "data-table", title: "Keyboard shortcuts",
        description: "The default keymap with J and K added. The table resolves keys only while it has focus, and the highlight moves on to the next page after the last row.",
        file: "data_table/keyboard.rs", height: 440,
    }
    PropertyGridInspector => property_grid::inspector {
        id: "property-grid/inspector", component: "property-grid", title: "Shape inspector",
        description: "The properties of a selected shape under collapsible categories: numbers with units, colours, a choice, switches and checkboxes, a read-only id and an opacity slider the app draws itself. A dot and a bold name mark values that differ from their default, with a button to reset them.",
        file: "property_grid/inspector.rs", height: 420,
    }
    PropertyGridFilterSort => property_grid::filter_sort {
        id: "property-grid/filter-sort", component: "property-grid", title: "Filter and sort",
        description: "The box filters properties by name and the buttons switch between categories and one alphabetical list. This one starts sorted by name and filtered to colours.",
        file: "property_grid/filter_sort.rs", height: 360,
    }
    PropertyGridValidation => property_grid::validation {
        id: "property-grid/validation", component: "property-grid", title: "Validation and reset",
        description: "An invalid port keeps its draft and shows why under the rows, without changing the value. Escape puts back the old value, and the reset buttons go back to the defaults.",
        file: "property_grid/validation.rs", height: 360,
    }
    PropertyGridKeyboard => property_grid::keyboard {
        id: "property-grid/keyboard", component: "property-grid", title: "Keyboard shortcuts",
        description: "The default keymap with J and K added. The grid resolves keys only while it has focus.",
        file: "property_grid/keyboard.rs", height: 480,
    }
    JsonViewerApiResponse => json_viewer::api_response {
        id: "json-viewer/api-response", component: "json-viewer", title: "API response",
        description: "A response opened two levels deep, with values coloured by type, a link too long to fit and a context menu on every row.",
        file: "json_viewer/api_response.rs", height: 400, task: true,
    }
    JsonViewerSearch => json_viewer::search {
        id: "json-viewer/search", component: "json-viewer", title: "Search",
        description: "Searching keys and values highlights every match and opens its parents. The arrows step through matches, and the filter shows only them.",
        file: "json_viewer/search.rs", height: 360, task: true,
    }
    JsonViewerCopy => json_viewer::copy {
        id: "json-viewer/copy", component: "json-viewer", title: "Copy values and paths",
        description: "The highlighted row has buttons to copy its value as JSON or its path, and the context menu does the same. The app writes the text to the clipboard.",
        file: "json_viewer/copy.rs", height: 440, task: true,
    }
    JsonViewerKeyboard => json_viewer::keyboard {
        id: "json-viewer/keyboard", component: "json-viewer", title: "Keyboard shortcuts",
        description: "The default keymap with J and K added. The viewer resolves keys only while it has focus.",
        file: "json_viewer/keyboard.rs", height: 440, task: true,
    }
    JsonViewerLargeDocument => json_viewer::large_document {
        id: "json-viewer/large-document", component: "json-viewer", title: "Large document",
        description: "1,500 orders, over 15,000 nodes. Only the rows in view are built, so expanding everything stays quick.",
        file: "json_viewer/large_document.rs", height: 440, task: true,
    }
    JsonViewerParseErrors => json_viewer::parse_errors {
        id: "json-viewer/parse-errors", component: "json-viewer", title: "Parse errors",
        description: "JSON text is parsed as it is typed. Invalid text shows the problem and where it is in place of the rows.",
        file: "json_viewer/parse_errors.rs", height: 200, task: true,
    }
    VirtualListLarge => virtual_list::large {
        id: "virtual-list/large", component: "virtual-list", title: "100,000 rows",
        description: "A hundred thousand orders in a multiple selection. Only the rows in view are built, so scrolling stays smooth.",
        file: "virtual_list/large.rs", height: 340,
    }
    VirtualListStreaming => virtual_list::streaming {
        id: "virtual-list/streaming", component: "virtual-list", title: "Streaming feed",
        description: "Simulated sensors send readings through the list's channel, using futures-timer so they also run in the browser. The list follows the newest reading until you scroll up, and keeps the newest thousand.",
        file: "virtual_list/streaming.rs", height: 340, subscription: true,
    }
    VirtualListKeyboard => virtual_list::keyboard {
        id: "virtual-list/keyboard", component: "virtual-list", title: "Keyboard shortcuts",
        description: "The default keymap with J and K added. The list resolves keys only while it has focus.",
        file: "virtual_list/keyboard.rs", height: 360,
    }
    LogViewerLive => log_viewer::live {
        id: "log-viewer/live", component: "log-viewer", title: "Live log",
        description: "A simulated server writes lines through the log's channel, using futures-timer so it also runs in the browser. Scroll up to stop following, then jump back to the newest line.",
        file: "log_viewer/live.rs", height: 420, subscription: true, task: true,
    }
    LogViewerFiltering => log_viewer::filtering {
        id: "log-viewer/filtering", component: "log-viewer", title: "Levels and search",
        description: "Trace and debug lines are hidden and long lines wrap. The search marks every match, and Enter or the arrows move between them.",
        file: "log_viewer/filtering.rs", height: 420, task: true,
    }
    LogViewerKeyboard => log_viewer::keyboard {
        id: "log-viewer/keyboard", component: "log-viewer", title: "Keyboard shortcuts",
        description: "The default keymap with J and K added. The log resolves keys while any part of it has focus, and copying hands the text to the app.",
        file: "log_viewer/keyboard.rs", height: 440, task: true,
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
    ShowcaseDemo => showcase::demo {
        id: "showcase/demo", component: "showcase", title: "Operations console",
        description: "Every component working together in one app: jobs streamed from a simulated worker, a command palette, menus, dialogs, toasts and settings.",
        file: "showcase/demo/mod.rs", height: 480, subscription: true, task: true, theme: true, edge: true,
    }
}
