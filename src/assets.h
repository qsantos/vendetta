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

#ifndef ASSETS_H
#define ASSETS_H

typedef struct assets assets_t;

#include <SFML/Graphics.h>

struct assets
{
	sfFont* font;

	char**      filenames;
	sfTexture** textures;

	size_t     n_sprites;
	size_t     a_sprites;
	sfSprite** sprites;
};

void assets_init(assets_t* a);
void assets_exit(assets_t* a);

sfTexture* assets_loadImage(assets_t* a, const char* filename);
int        assets_spriteId (assets_t* a, const char* filename);
sfSprite*  assets_sprite   (assets_t* a, const char* filename);

#endif
