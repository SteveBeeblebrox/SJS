console.log('Hello from worker')

await new Promise(r => setTimeout(r, 2000));
console.log('Later!')
// self.close()