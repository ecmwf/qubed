![Static Badge](https://img.shields.io/badge/ESEE-Production_Chain-blue?style=flat&label=ESEE&link=github.com%2Fecmwf)
![Static Badge](https://img.shields.io/badge/ESEE-Data_Provision-purple?style=flat&label=ESEE&link=github.com%2Fecmwf)
![Static Badge](https://img.shields.io/badge/ESEE-User_Interaction-green?style=flat&label=ESEE&link=github.com%2Fecmwf)
![Static Badge](https://img.shields.io/badge/ESEE-Foundation-orange?style=flat&label=ESEE&link=github.com%2Fecmwf)


# Q<sup>3</sup> Quick Querying of Qubes


## How this works

All valid MARS requests start with `class=something` so at the root of this viewer you are presented with options for what class to choose. **You are allowed to choose multiple values.** After you have selected values, click `next`. At each subsequent step the backend checks any possible match between the values you have chosen and the fdb schema, sometimes this will mean that multiple keys can be selected such as [here](http://136.156.129.226/app/index.html?class=od&expver=0001&stream=enfo,oper&date=20241004&time=1226&domain=d,g&type=fc,pf&levtype=pl,sfc&step=1) where both `levellist` and `quantile` are valid keys to continue the query, again you can select both.

This is not implemented yet but the idea is that you will be able to generate a polytope query from this tool.
 

## [Test instance 1](http://136.156.129.226/app/index.html)

This one is not actually looking at any data, it just looks at the default `fdb_schema_file` from fdb and the mars `language.yaml` in metkit. The former provides a way to determine roughly what requests are valid while the later allows us to fill in suggested values for keys such as `class`, `expverr`, `stream` etc.

<img width="1179" alt="image" src="https://github.com/user-attachments/assets/83ffe097-8526-4e94-b8ea-6ac630821233">
