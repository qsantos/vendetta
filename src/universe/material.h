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

#ifndef U_MATERIAL_H
#define U_MATERIAL_H

typedef struct kindOf_material kindOf_material_t;

#include "../graphics.h"
#include "status.h"

struct kindOf_material
{
	char* name;

	int skill;

	char edible;
	float eatBonus[N_STATUSES];

	int icon_sprite;
	int icon_index;
};

void kindOf_material_init(kindOf_material_t* m);
void kindOf_material_exit(kindOf_material_t* m);

void kindOf_material_icon(kindOf_material_t* m, graphics_t* g, const char* filename, int idx);

#endif
