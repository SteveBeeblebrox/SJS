//console.log(`Hello sjs v${0}`);
//console.log(await (await fetch('https://g.co')).text())

import { getTSVersion } from "./deno.ts";


console.log(globalThis['system'].version)

console.log(`Args: ${system.args}`)

console.log(getTSVersion())


//worker still expects (url,{deno:{}}) not sjs CHANGE in runtime/js/11_workers.js line 98 and others
// new Worker(import.meta.resolve('./worker.js'),{type:'module',ext:{
//     permissions: 'inherit',
//     persistent: false
// }})

// new Worker(import.meta.resolve('./worker.js'),{type:'module'})

// system.test('basic test', function() {
//     throw new Error('ahhhhh!')
// });

// // localStorage.setItem('foo',1)
// console.log(localStorage.getItem('foo'))