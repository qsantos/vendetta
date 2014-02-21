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

#include "projectile.h"

#include <stdio.h>

void kindOf_projectile_init(kindOf_projectile_t* p)
{
	p->sprite = -1;
	p->event  = -1;

	p->width  = 0;
	p->height = 0;
}

void kindOf_projectile_exit(kindOf_projectile_t* p)
{
	(void) p;
}

void kindOf_projectile_sprite(kindOf_projectile_t* p, assets_t* a, const char* filename)
{
	char s[1024];
	snprintf(s, 1024, "data/projectiles/%s", filename);
	int id = assets_spriteId(a, s);

	p->sprite = id;

	sfIntRect rect = sfSprite_getTextureRect(a->sprites[id]);
	p->width  = rect.width  / 3;
	p->height = rect.height / 4;
}
