create table aka_name (
    id integer not null,
    person_id integer not null,
    name varchar(255) not null,
    imdb_index varchar(3) not null,
    name_pcode_cf varchar(11) not null,
    name_pcode_nf varchar(11) not null,
    surname_pcode varchar(11) not null,
    md5sum varchar(65) not null,
    primary key (id)
);

create table aka_title (
    id integer not null,
    movie_id integer not null,
    title varchar(255) not null,
    imdb_index varchar(4) not null,
    kind_id integer not null,
    production_year integer not null,
    phonetic_code varchar(5) not null,
    episode_of_id integer not null,
    season_nr integer not null,
    episode_nr integer not null,
    note varchar(72) not null,
    md5sum varchar(32) not null,
    primary key (id)
);

create table cast_info (
    id integer not null,
    person_id integer not null,
    movie_id integer not null,
    person_role_id integer not null,
    note varchar(255) not null,
    nr_order integer not null,
    role_id integer not null,
    primary key (id)
);

create table char_name (
    id integer not null,
    name varchar(255) not null,
    imdb_index varchar(2) not null,
    imdb_id integer not null,
    name_pcode_nf varchar(5) not null,
    surname_pcode varchar(5) not null,
    md5sum varchar(32) not null,
    primary key (id)
);

create table comp_cast_type (
    id integer not null,
    kind varchar(32) not null,
    primary key (id)
);

create table company_name (
    id integer not null,
    name varchar(255) not null,
    country_code varchar(6) not null,
    imdb_id integer not null,
    name_pcode_nf varchar(5) not null,
    name_pcode_sf varchar(5) not null,
    md5sum varchar(32) not null,
    primary key (id)
);

create table company_type (
    id integer not null,
    kind varchar(32) not null,
    primary key (id)
);

create table complete_cast (
    id integer not null,
    movie_id integer not null,
    subject_id integer not null,
    status_id integer not null,
    primary key (id)
);

create table info_type (
    id integer not null,
    info varchar(32) not null,
    primary key (id)
);

create table keyword (
    id integer not null,
    keyword varchar(255) not null,
    phonetic_code varchar(5) not null,
    primary key (id)
);

create table kind_type (
    id integer not null,
    kind varchar(15) not null,
    primary key (id)
);

create table link_type (
    id integer not null,
    link varchar(32) not null,
    primary key (id)
);

create table movie_companies (
    id integer not null,
    movie_id integer not null,
    company_id integer not null,
    company_type_id integer not null,
    note varchar(30) not null,
    primary key (id)
);

create table movie_info_idx (
    id integer not null,
    movie_id integer not null,
    info_type_id integer not null,
    info varchar(255) not null,
    note varchar(30) not null,
    primary key (id)
);

create table movie_keyword (
    id integer not null,
    movie_id integer not null,
    keyword_id integer not null,
    primary key (id)
);

create table movie_link (
    id integer not null,
    movie_id integer not null,
    linked_movie_id integer not null,
    link_type_id integer not null,
    primary key (id)
);

create table name (
    id integer not null,
    name varchar(255) not null,
    imdb_index varchar(9) not null,
    imdb_id integer not null,
    gender varchar(1) not null,
    name_pcode_cf varchar(5) not null,
    name_pcode_nf varchar(5) not null,
    surname_pcode varchar(5) not null,
    md5sum varchar(32) not null,
    primary key (id)
);

create table role_type (
    id integer not null,
    role varchar(32) not null,
    primary key (id)
);

create table title (
    id integer not null,
    title varchar(255) not null,
    imdb_index varchar(5) not null,
    kind_id integer not null,
    production_year integer not null,
    imdb_id integer not null,
    phonetic_code varchar(5) not null,
    episode_of_id integer not null,
    season_nr integer not null,
    episode_nr integer not null,
    series_years varchar(49) not null,
    md5sum varchar(32) not null,
    primary key (id)
);

create table movie_info (
    id integer not null,
    movie_id integer not null,
    info_type_id integer not null,
    info varchar(255) not null,
    note varchar(255) not null,
    primary key (id)
);

create table person_info (
    id integer not null,
    person_id integer not null,
    info_type_id integer not null,
    info varchar(255) not null,
    note varchar(255) not null,
    primary key (id)
);

COPY title FROM 'data/imdb/title.csv' DELIMITER ',';
COPY movie_companies FROM 'data/imdb/movie_companies.csv' DELIMITER ',';
COPY person_info FROM 'data/imdb/person_info.csv' DELIMITER ',';
COPY name FROM 'data/imdb/name.csv' DELIMITER ',';
COPY movie_link FROM 'data/imdb/movie_link.csv' DELIMITER ',';
COPY char_name FROM 'data/imdb/char_name.csv' DELIMITER ',';
COPY cast_info FROM 'data/imdb/cast_info.csv' DELIMITER ',';
COPY company_name FROM 'data/imdb/company_name.csv' DELIMITER ',';
COPY company_type FROM 'data/imdb/company_type.csv' DELIMITER ',';
COPY role_type FROM 'data/imdb/role_type.csv' DELIMITER ',';
COPY keyword FROM 'data/imdb/keyword.csv' DELIMITER ',';
COPY link_type FROM 'data/imdb/link_type.csv' DELIMITER ',';
COPY movie_keyword FROM 'data/imdb/movie_keyword.csv' DELIMITER ',';
COPY movie_info_idx FROM 'data/imdb/movie_info_idx.csv' DELIMITER ',';
COPY movie_info FROM 'data/imdb/movie_info.csv' DELIMITER ',';
COPY info_type FROM 'data/imdb/info_type.csv' DELIMITER ',';
COPY kind_type FROM 'data/imdb/kind_type.csv' DELIMITER ',';
COPY aka_title FROM 'data/imdb/aka_title.csv' DELIMITER ',';
COPY aka_name FROM 'data/imdb/aka_name.csv' DELIMITER ',';
COPY comp_cast_type FROM 'data/imdb/comp_cast_type.csv' DELIMITER ',';
COPY complete_cast  FROM 'data/imdb/complete_cast.csv'  DELIMITER ',';

