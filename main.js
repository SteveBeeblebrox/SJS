//console.log(`Hello sjs v${0}`);
//console.log(await (await fetch('https://g.co')).text())

//import { copy } from "https://deno.land/std@0.215.0/fs/copy.ts";


console.log(globalThis['system'].version)

console.log(`Args: ${system.args}`)


system.test('basic test', function() {
    throw new Error('ahhhhh!')
});

// localStorage.setItem('foo',1)
console.log(localStorage.getItem('foo'))