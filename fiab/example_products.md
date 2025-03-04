


Simplest possible product
- one field: 2 metre temperature
- all models that output param=2t would work
- may also have a lead time range specified from

So we could say "here are all the models with param=2t with lead times in the specified interval"

quantiles
  param:
  float range from 0 - 100

threshold:
  "give me 2 metre temperature values that are above this threshold"


  product requrements can be specified as a set of:
    params: one or more params
    levels: one or more or all
    time:
      - product could be specific to a particular time
      - could require at least a months worth of data


make some fake models that have:
 - fewer params
 - continous times vs steps of 6 hours
 -


Could also represent what data is currently cached on disk and be able to then tell the use what stuff they can generate really fast.

API want:
  - way to get axis span like what params exist
  -
