/* The page's only script: progressive enhancement that morphs a card
   into its popover with a view transition. The popover itself opens
   natively (popovertarget) without it. */
document.addEventListener('click', (event) => {
  const button = event.target.closest('[popovertarget]');
  if (!button || !document.startViewTransition) return;
  const popover = document.getElementById(button.getAttribute('popovertarget'));
  const card = button.closest('.card');
  if (!popover || !card) return;
  event.preventDefault();
  card.style.viewTransitionName = 'chips-expand';
  const transition = document.startViewTransition(() => {
    card.style.viewTransitionName = '';
    popover.style.viewTransitionName = 'chips-expand';
    popover.showPopover();
  });
  transition.finished.finally(() => {
    popover.style.viewTransitionName = '';
  });
});
