globalThis.addEventListener('unload',()=>console.log('exiting!'));

const IMPL = '/home/sianabeeblebrox/libsdsnoop.js';
const DUMMY = './dummyadd.js';

(async function() {
    try {
        console.log('Entered Script');
        const {add} = await import(IMPL);
        console.log('Loaded lib');
        console.log(add(1,2));
    } catch(e) {
        console.error(e);
    }
})()

// import {add} from '/home/sianabeeblebrox/libsdsnoop.js';
