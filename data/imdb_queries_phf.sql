SELECT count(*)
FROM movie_companies mc,
     title t
WHERE  mc.movie_id = t.id
  AND t.production_year = 2012;


SELECT COUNT(*)
FROM
    (SELECT mc.movie_id, mc.company_id, mc.company_type_id
     FROM movie_companies mc, title t
     WHERE mc.movie_id = t.id
       AND t.production_year = 2000
    ) tm,
    (SELECT mi.movie_id, mi.info_type_id
     FROM movie_info mi, info_type it1
     WHERE mi.info_type_id = it1.id
    ) mi1,
    (SELECT mi_idx.movie_id, mi_idx.info_type_id
     FROM movie_info_idx mi_idx, info_type it2
     WHERE mi_idx.info_type_id = it2.id
    ) mi2,
    company_name cn,
    company_type ct
WHERE cn.id = tm.company_id
  AND ct.id = tm.company_type_id
  AND mi1.movie_id = tm.movie_id
  AND mi2.movie_id = tm.movie_id;

SELECT COUNT(*)
FROM
    (SELECT mc.movie_id, mc.company_id, mc.company_type_id
     FROM movie_companies mc, title t
     WHERE mc.movie_id = t.id
       AND t.production_year = 2000
    ) tm,
    company_name cn,
    company_type ct,
    movie_info mi,
    info_type it1,
    movie_info_idx mi_idx,
    info_type it2
WHERE cn.id = tm.company_id
  AND ct.id = tm.company_type_id
  AND mi.movie_id = tm.movie_id
  AND mi.info_type_id = it1.id
  AND mi_idx.movie_id = tm.movie_id
  AND mi_idx.info_type_id = it2.id;


SELECT COUNT(*)
FROM
    (SELECT t.id, t.kind_id, mc.company_id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.id = mc.movie_id
       AND t.production_year = 2005
    ) tm,
    company_name cn,
    company_type ct,
    kind_type kt,
    movie_info mi,
    info_type it2,
    movie_info_idx miidx,
    info_type it
WHERE cn.id = tm.company_id
  AND ct.id = tm.company_type_id
  AND kt.id = tm.kind_id
  AND mi.movie_id = tm.id
  AND it2.id = mi.info_type_id
  AND miidx.movie_id = tm.id
  AND it.id = miidx.info_type_id
  AND mi.movie_id = miidx.movie_id;



SELECT COUNT(*)
FROM
    (SELECT t.id, t.kind_id,
            mi.info_type_id,
            mi_idx.info_type_id,
            mk.keyword_id
     FROM title t,
          kind_type kt,
          movie_info mi,
          movie_info_idx mi_idx,
          movie_keyword mk
     WHERE t.production_year = 2012
       AND kt.id = t.kind_id
       AND mi.movie_id = t.id
       AND mi_idx.movie_id = t.id
       AND mk.movie_id = t.id
       AND mi.movie_id = mi_idx.movie_id
       AND mk.movie_id = mi.movie_id
       AND mk.movie_id = mi_idx.movie_id
    ) tm,
    info_type it1,
    info_type it2,
    keyword k
WHERE it1.id = tm.info_type_id
  AND it2.id = tm.info_type_id
  AND k.id = tm.keyword_id;


SELECT COUNT(*)
FROM
    (SELECT t.id, t.kind_id
     FROM title t, kind_type kt
     WHERE t.production_year = 2012
       AND kt.id = t.kind_id
    ) tm,
    (SELECT mi.movie_id, mi.info_type_id
     FROM movie_info mi
    ) mi1,
    (SELECT mi_idx.movie_id, mi_idx.info_type_id
     FROM movie_info_idx mi_idx
    ) mi2,
    (SELECT mk.movie_id, mk.keyword_id
     FROM movie_keyword mk
    ) mk1,
    info_type it1,
    info_type it2,
    keyword k
WHERE mi1.movie_id = tm.id
  AND mi2.movie_id = tm.id
  AND mk1.movie_id = tm.id
  AND it1.id = mi1.info_type_id
  AND it2.id = mi2.info_type_id
  AND k.id = mk1.keyword_id
  AND mi1.movie_id = mi2.movie_id
  AND mi1.movie_id = mk1.movie_id;

SELECT COUNT(*)
FROM title t,
     movie_companies mc,
     aka_title at,
     movie_info mi,
     info_type it1,
     movie_keyword mk,
     keyword k,
     company_name cn,
     company_type ct
WHERE t.production_year = 2001
  AND it1.id = 5
  AND k.id = 10
  AND ct.id = 2
  AND t.id = mc.movie_id
  AND t.id = at.movie_id
  AND t.id = mi.movie_id
  AND t.id = mk.movie_id
  AND it1.id = mi.info_type_id
  AND k.id = mk.keyword_id
  AND cn.id = mc.company_id
  AND ct.id = mc.company_type_id;


SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2000
       AND t.production_year <= 2003
       AND mc.id >= 1
       AND mc.id <= 350000
       AND t.id = mc.movie_id
    ) tm,
    aka_title at,
    movie_info mi,
    info_type it1,
    movie_keyword mk,
    keyword k,
    company_name cn,
    company_type ct
WHERE at.id >= 1
  AND at.id <= 350000
  AND mi.id >= 1
  AND mi.id <= 350000
  AND mk.id >= 1
  AND mk.id <= 350000
  AND at.movie_id = tm.id
  AND mi.movie_id = tm.id
  AND mk.movie_id = tm.id
  AND it1.id = mi.info_type_id
  AND k.id = mk.keyword_id
  AND cn.id = tm.company_id
  AND ct.id = tm.company_type_id;


SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2000
       AND t.production_year <= 2001
       AND mc.id >= 1
       AND mc.id <= 200000
       AND t.id = mc.movie_id
    ) tm,
    aka_title at,
    movie_info mi,
    info_type it1,
    movie_keyword mk,
    keyword k,
    company_name cn,
    company_type ct
WHERE at.id >= 1
  AND at.id <= 200000
  AND mi.id >= 1
  AND mi.id <= 200000
  AND mk.id >= 1
  AND mk.id <= 200000

  AND at.movie_id = tm.id
  AND mi.movie_id = tm.id
  AND mk.movie_id = tm.id
  AND it1.id = mi.info_type_id
  AND k.id = mk.keyword_id
  AND cn.id = tm.company_id
  AND ct.id = tm.company_type_id;



SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2000
       AND t.production_year <= 2001
       AND mc.id >= 1
       AND mc.id <= 200000
       AND t.id = mc.movie_id
    ) tm,
    company_type ct,
    movie_info_idx mi_idx,
    info_type it
WHERE ct.id = tm.company_type_id
  AND mi_idx.movie_id = tm.id
  AND it.id = mi_idx.info_type_id
  AND mi_idx.id >= 1
  AND mi_idx.id <= 200000
  AND it.id >= 1
  AND it.id <= 100;


SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 1998
       AND t.production_year <= 2003
       AND t.id = mc.movie_id
    ) tm,
    company_type ct,
    movie_info_idx mi_idx,
    info_type it
WHERE ct.id = tm.company_type_id
  AND mi_idx.movie_id = tm.id
  AND it.id = mi_idx.info_type_id;

SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 1998
       AND t.production_year <= 2003
       AND t.id = mc.movie_id
    ) tm,
    company_name cn,
    movie_keyword mk,
    keyword k
WHERE cn.id = tm.company_id
  AND mk.movie_id = tm.id
  AND k.id = mk.keyword_id;

SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2000
       AND t.id = mc.movie_id
    ) tm,
    company_name cn,
    movie_keyword mk,
    keyword k
WHERE cn.id = tm.company_id
  AND mk.movie_id = tm.id
  AND k.id = mk.keyword_id;

SELECT COUNT(*)
FROM
    (SELECT t.id, mi_idx.info_type_id
     FROM title t, movie_info_idx mi_idx
     WHERE t.production_year >= 2006
       AND t.id = mi_idx.movie_id
    ) tm,
    movie_keyword mk,
    keyword k,
    info_type it
WHERE mk.movie_id = tm.id
  AND k.id = mk.keyword_id
  AND it.id = tm.info_type_id;

SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2006
       AND t.production_year <= 2007
       AND t.id = mc.movie_id
    ) tm,
    movie_info mi,
    info_type it,
    company_type ct
WHERE mi.movie_id = tm.id
  AND ct.id = tm.company_type_id
  AND it.id = mi.info_type_id;


SELECT COUNT(*)
FROM
    (SELECT t.id
     FROM title t
     WHERE t.production_year >= 2011
       AND t.production_year <= 2012
    ) tm,
    movie_keyword mk,
    keyword k,
    cast_info ci,
    name n
WHERE mk.movie_id = tm.id
  AND ci.movie_id = tm.id
  AND k.id = mk.keyword_id
  AND n.id = ci.person_id
  AND ci.movie_id = mk.movie_id;


SELECT COUNT(*)
FROM
    (SELECT t.id
     FROM title t
     WHERE t.production_year >= 1980
       AND t.production_year <= 1982
    ) tm,
    movie_link ml,
    link_type lt,
    cast_info ci,
    name n,
    aka_name an,
    person_info pi,
    info_type it
WHERE ml.linked_movie_id = tm.id
  AND ci.movie_id = tm.id
  AND lt.id = ml.link_type_id
  AND n.id = ci.person_id
  AND an.person_id = n.id
  AND pi.person_id = n.id
  AND it.id = pi.info_type_id;

SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2000
       AND t.production_year <= 2001
       AND t.id = mc.movie_id
    ) tm,
    company_name cn,
    cast_info ci,
    role_type rt,
    name n1,
    aka_name an1
WHERE cn.id = tm.company_id
  AND ci.movie_id = tm.id
  AND rt.id = ci.role_id
  AND n1.id = ci.person_id
  AND an1.person_id = n1.id
  AND an1.person_id = ci.person_id;


SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2005
       AND t.production_year <= 2006
       AND t.id = mc.movie_id
    ) tm,
    company_name cn,
    cast_info ci,
    role_type rt,
    name n,
    char_name chn,
    aka_name an
WHERE cn.id = tm.company_id
  AND ci.movie_id = tm.id
  AND rt.id = ci.role_id
  AND n.id = ci.person_id
  AND chn.id = ci.person_role_id
  AND an.person_id = n.id
  AND an.person_id = ci.person_id;


SELECT COUNT(*)
FROM
    (SELECT t.id, t.kind_id, mc.company_id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2000
       AND t.production_year <= 2001
       AND t.id = mc.movie_id
    ) tm,
    company_name cn,
    company_type ct,
    kind_type kt,
    movie_info mi,
    info_type it2,
    movie_info_idx miidx,
    info_type it
WHERE cn.id = tm.company_id
  AND ct.id = tm.company_type_id
  AND kt.id = tm.kind_id
  AND mi.movie_id = tm.id
  AND it2.id = mi.info_type_id
  AND miidx.movie_id = tm.id
  AND it.id = miidx.info_type_id
  AND mi.movie_id = miidx.movie_id;


SELECT COUNT(*)
FROM
    (SELECT t.id, t.kind_id
     FROM title t
     WHERE t.production_year >= 2011
       AND t.production_year <= 2012
    ) tm,
    kind_type kt,
    movie_info mi,
    info_type it1,
    movie_info_idx mi_idx,
    info_type it2,
    movie_keyword mk,
    keyword k
WHERE kt.id = tm.kind_id
  AND mi.movie_id = tm.id
  AND mi_idx.movie_id = tm.id
  AND mk.movie_id = tm.id
  AND it1.id = mi.info_type_id
  AND it2.id = mi_idx.info_type_id
  AND k.id = mk.keyword_id
  AND mi.movie_id = mi_idx.movie_id
  AND mk.movie_id = mi.movie_id
  AND mk.movie_id = mi_idx.movie_id;

SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id, mc.company_type_id
     FROM title t, movie_companies mc
     WHERE t.production_year >= 2001
       AND t.production_year <= 2002
       AND mc.id >= 1
       AND mc.id <= 200000
       AND t.id = mc.movie_id
    ) tm,
    aka_title at,
    movie_info mi,
    info_type it1,
    movie_keyword mk,
    keyword k,
    company_name cn,
    company_type ct
WHERE at.movie_id = tm.id
  AND mi.movie_id = tm.id
  AND mk.movie_id = tm.id
  AND it1.id = mi.info_type_id
  AND k.id = mk.keyword_id
  AND cn.id = tm.company_id
  AND ct.id = tm.company_type_id
  AND mi.movie_id = mk.movie_id
  AND at.movie_id = mi.movie_id;


SELECT COUNT(*)
FROM
    (SELECT t.id
     FROM title t
     WHERE t.episode_nr >= 50
       AND t.episode_nr <= 99
       AND t.production_year >= 2000
       AND t.production_year <= 2010
    ) tm,
    cast_info ci,
    name n,
    aka_name an,
    movie_keyword mk,
    keyword k,
    movie_companies mc,
    company_name cn
WHERE ci.movie_id = tm.id
  AND n.id = ci.person_id
  AND an.person_id = n.id
  AND mk.movie_id = tm.id
  AND k.id = mk.keyword_id
  AND mc.movie_id = tm.id
  AND cn.id = mc.company_id
  AND ci.movie_id = mc.movie_id
  AND ci.movie_id = mk.movie_id
  AND mc.movie_id = mk.movie_id;

SELECT COUNT(*)
FROM
    (SELECT t.id, mc.company_id
     FROM title t, movie_companies mc
     WHERE t.production_year = 2000
       AND t.id = mc.movie_id
    ) tm,
    company_name cn,
    movie_keyword mk,
    keyword k,
    cast_info ci,
    name n
WHERE cn.id = tm.company_id
  AND mk.movie_id = tm.id
  AND k.id = mk.keyword_id
  AND ci.movie_id = tm.id
  AND n.id = ci.person_id
  AND ci.movie_id = mk.movie_id;