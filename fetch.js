console.log(await(await fetch('https://g.co')).text())
console.log(await(await fetch('https://expired.badssl.com')).text())