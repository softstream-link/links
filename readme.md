

![alt text](test.drawio.svg)



# TODO

* add protocols

    atp - aquis trading platform - https://www.aquis.eu/markets/documents#technical
    boe - https://cdn.cboe.com/resources/membership/Cboe_US_Equities_BOE_Specification.pdf
    fixp - https://www.cmegroup.com/confluence/display/EPICSANDBOX/iLink+3+Binary+Order+Entry
    mit - london stock exchange group - https://docs.londonstockexchange.com/sites/default/files/   documents/mit203_-_native_trading_gateway_specification_-_issue_13.2.pdf
    optiq - 
    https://connect2.euronext.com/sites/default/files/it-documentation/Optiq%20OEG%20SBE%20Messages%20-%20Interface%20Specification%20-%20Euronext%20Cash%20and%20Derivatives%20Markets%20-%20External%20-%20v5.24.0%20%2BTC.pdf

    QAUISATP
    BATSBOE
    CDG
    ETI
    EURONEXT CCGD
    Exture Korea
    FIX 4.2 4.5
    FIXP
    Nnf NSE
    OMnet
    OMNetIF
    OPTIQSBE
    XETI
    

run tests
cargo nextest run

TODO performance ideas to investigate
    try enums eq(...) with const instead of funct
    eliminate ouch appendix to see how much Option<> has effect on deser
    s use match in stead of if in the der logic of the Option