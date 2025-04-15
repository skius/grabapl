function has<T>(array: T[], value: T): boolean {
  return array.includes(value);
}

function add<T>(array: T[], value: T): T[] {
  if (!has(array, value)) {
    array.push(value);
  }
  return array;
}

function remove<T>(array: T[], value: T): T[] {
  const idx = array.indexOf(value);
  if (idx >= 0) array.splice(array.indexOf(value), 1);
  return array;
}

export const ArraySet = {
  has,
  add,
  remove,
};
