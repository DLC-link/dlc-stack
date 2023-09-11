import { bufferCV, bufferCVFromString, callReadOnlyFunction, cvToValue, } from '@stacks/transactions';
import { hexToBytes as hexToBytesMS } from 'micro-stacks/common';
export const callReadOnly = async (txOptions, dirDepth = 3) => {
    const transaction = await callReadOnlyFunction(txOptions);
    // console.log('[readOnly] transaction:');
    // console.dir(transaction, { depth: dirDepth });
    // console.log('[readOnly] cvToValue():');
    // console.dir(cvToValue(transaction), { depth: dirDepth });
    return { cv: transaction, cvToValue: cvToValue(transaction) };
};
/**
 * Shift a number's decimal place by a specified amount.
 *
 * @param {number} value - The value to be shifted.
 * @param {number} shift - The number of decimal places to shift the value by.
 * @param {boolean} [unshift] - boolean - if true, the value will be unshifted.
 * @returns The resulting shifted number.
 */
export function customShiftValue(value, shift, unshift) {
    return unshift ? value / 10 ** shift : value * 10 ** shift;
}
/**
 * The function takes a number, shifts the decimal place by two
 * @param {number} value - number - the value to be shifted e.g. 160000
 * @returns e.g. 1600.00
 */
export function fixedTwoDecimalShift(value) {
    return customShiftValue(value, 2, true).toFixed(2);
}
export function hex2ascii(hexx) {
    if (!hexx)
        return '';
    var hex = hexx.toString();
    var str = '';
    for (var i = 2; i < hex.length; i += 2)
        str += String.fromCharCode(parseInt(hex.substr(i, 2), 16));
    return str;
}
export function timestampToDate(timestamp) {
    if (!timestamp)
        return '';
    return new Date(timestamp * 1000).toLocaleString();
}
// The following are from the RedStone helper library for Stacks.
/**
 * Utility conversion function that can take both 0x prefixed
 * and unprefixed hex strings.
 * @param hex
 * @returns Uint8Array
 */
export function hexToBytes(hex) {
    return hexToBytesMS(hex.substring(0, 2) === '0x' ? hex.substring(2) : hex);
}
/**
 * Shifts the price value according to RedStone serialisation.
 * @param value
 * @returns shifted value
 */
export function shiftPriceValue(value) {
    return Math.round(value * 10 ** 8);
}
/**
 * It takes a string and returns a ClarityValue based on length
 * @param {string} uuid - The UUID to convert to a ClarityValue
 * @returns A ClarityValue
 */
export function uuidToCV(uuid) {
    return uuid.length > 8 ? bufferCV(hexToBytes(uuid)) : bufferCVFromString(uuid);
}
export function uuidResponseToString(uuid) {
    return uuid.length > 8 ? uuid : hex2ascii(uuid);
}
// This cuts the UUID down to a more manageable size for display
// Removes the '0x' prefix and shows the last and first 4 characters
export function formatUUID(uuid) {
    return `${uuid.slice(2, 6)}...${uuid.slice(-4)}`;
}
