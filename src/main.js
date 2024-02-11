//console.log(`Hello sjs v${0}`);
//console.log(await (await fetch('https://g.co')).text())
console.log(globalThis['system'].version)

console.log(`Args: ${system.args}`)


system.test('basic test', function() {
    throw new Error('ahhhhh!')
});