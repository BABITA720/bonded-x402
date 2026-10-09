const i = require('./target/idl/bonded_x402.json');
i.instructions.forEach(x => {
  console.log(x.name, '|', x.accounts.map(a => a.name).join(','), '|', x.args.map(a => a.name + ':' + JSON.stringify(a.type)).join(','));
});
