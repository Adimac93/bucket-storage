import { writable } from "svelte/store";

const itemName = "auth-key";
export const key = writable(localStorage.getItem(itemName));
key.subscribe((k) => {
  localStorage.setItem(itemName, k);
});
