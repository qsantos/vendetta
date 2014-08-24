/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#ifndef U_COMPONENT_LIST_H
#define U_COMPONENT_LIST_H

struct inventory;

typedef struct component component_t;
typedef struct transform transform_t;

#define MATERIAL 0
#define ITEM     1

struct component
{
	char  is_item;
	int   id;
	float amount;
};

struct transform
{
	int n_req;
	component_t* req;

	int n_res;
	component_t* res;

	float rate;
};

void transform_init(transform_t* t);
void transform_exit(transform_t* t);
void transform_copy(transform_t* t, transform_t* from);

void transform_req(transform_t* t, char is_item, int id, float a);
void transform_res(transform_t* t, char is_item, int id, float a);
void transform_add(transform_t* t, transform_t* a, float r);

int   transform_check(transform_t* t, struct inventory* inv);
float transform_ratio(transform_t* t, struct inventory* inv, float max_ratio);
float transform_apply(transform_t* t, struct inventory* inv, float ratio);

int transform_is_req(transform_t* t, char is_item, int id);
int transform_is_res(transform_t* t, char is_item, int id);

#endif
