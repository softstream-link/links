

![alt text](test.drawio.svg)



# TODO

* add protocols

    atp - aquis trading platform - https://www.aquis.eu/markets/documents#technical
    boe - https://cdn.cboe.com/resources/membership/Cboe_US_Equities_BOE_Specification.pdf
    fixp - https://www.cmegroup.com/confluence/display/EPICSANDBOX/iLink+3+Binary+Order+Entry
    mit - london stock exchange group - https://docs.londonstockexchange.com/sites/default/files/   documents/mit203_-_native_trading_gateway_specification_-_issue_13.2.pdf
    optiq - https://connect2.euronext.com/sites/default/files/it-documentation/Euronext_Markets_-_Optiq_OEG_Client_Specifications_-_SBE_Interface_-_v4.1.0%5B1%5D_0.pdf

    

run tests
cargo nextest run

TODO performance ideas to investigate
    try enums eq(...) with const instead of funct
    eliminate ouch appendix to see how much Option<> has effect on deser
    s use match in stead of if in the der logic of the Option