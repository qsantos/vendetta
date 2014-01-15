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

#include "overlay.h"

#include <math.h>

#include "../util.h"

overlay_t* overlay_init(graphics_t* g)
{
	overlay_t* o = CALLOC(overlay_t, 1);

	   ov_build_init(&o->build);
	 swbuilding_init(&o->swbuilding,  g);
	    switems_init(&o->switems,     g);
	swmaterials_init(&o->swmaterials, g);
	   swskills_init(&o->swskills,    g);
	swequipment_init(&o->swequipment, g);

	return o;
}

void overlay_exit(overlay_t* o)
{
	swequipment_exit(&o->swequipment);
	   swskills_exit(&o->swskills);
	    switems_exit(&o->switems);
	swmaterials_exit(&o->swmaterials);
	 swbuilding_exit(&o->swbuilding);
	   ov_build_exit(&o->build);

	free(o);
}

void overlay_cursor(overlay_t* o, game_t* g)
{
	sfVector2i cursor = sfMouse_getPositionRenderWindow(g->g->render);
	sfIntRect rect = {0, 0, 24, 24};

	if (0);
	else if (   ov_build_cursor(&o->build,       g, cursor.x, cursor.y))
		rect.left = 4 * 24;
	else if ( swbuilding_cursor(&o->swbuilding,  g, cursor.x, cursor.y));
	else if (    switems_cursor(&o->switems,     g, cursor.x, cursor.y));
	else if (swmaterials_cursor(&o->swmaterials, g, cursor.x, cursor.y));
	else if (   swskills_cursor(&o->swskills,    g, cursor.x, cursor.y));
	else if (swequipment_cursor(&o->swequipment, g, cursor.x, cursor.y));
	else
	{
		sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, cursor, g->g->world_view);
		object_t* o = world_objectAt(g->w, pos.x, pos.y);
		if (o != NULL)
		{
			if (o->t == O_MINE)
				rect.left = 1 * 24;
			else if (o->t == O_BUILDING)
			{
				building_t* b = (building_t*) o;
				if (b->build_progress == 1)
					rect.left = 7 * 24;
				else
					rect.left = 4 * 24;
			}
		}
	}

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g->g, "cursors.png");
		sprite = g->g->sprites[id];
	}

	sfSprite_setTextureRect(sprite, rect);

	sfSprite_setPosition(sprite, (sfVector2f){cursor.x, cursor.y});
	sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
}

void overlay_draw(overlay_t* o, game_t* g)
{
	// statuses
	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfColor col = {255, 255, 255, 255};
		sfText_setColor(text, col);
		sfText_setFont(text, g->g->font);
		sfText_setCharacterSize(text, 15);
	}
	sfVector2u size = sfRenderWindow_getSize(g->g->render);
	float x = 10;
	float y = size.y - 10 - 30 * N_STATUSES;;
	for (int i = 0; i < N_STATUSES; i++)
	{
		float p = g->player->statuses[i] / 20;
		graphics_drawProgressBar(g->g, x, y, 150, 20, p);

		sfText_setUTF8(text, g->u->statuses[i].name); // TODO
		sfVector2f pos = {x+5, y};
		sfText_setPosition(text, pos);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		char buffer[1024];
		snprintf(buffer, 1024, "%.0f/%.0f", floor(g->player->statuses[i]), floor(20.));
		sfText_setUTF8(text, buffer);
		sfFloatRect rect = sfText_getLocalBounds(text);
		pos.x = x + 140 - rect.width;
		sfText_setPosition(text, pos);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		y += 30;
	}

	 swbuilding_draw(&o->swbuilding,  g, 1);
	    switems_draw(&o->switems,     g, 1);
	swmaterials_draw(&o->swmaterials, g, 1);
	   swskills_draw(&o->swskills,    g, 1);
	swequipment_draw(&o->swequipment, g, 1);

	ov_build_draw(&o->build, g, 1);

	overlay_cursor(g->o, g);

	char buffer[1024];
	snprintf(buffer, 1024, "FPS: %.0f", floor(g->g->fps));
	sfText_setUTF8(text, buffer);
	sfVector2f pos = {size.x - 80, size.y - 30};
	sfText_setPosition(text, pos);
	sfRenderWindow_drawText(g->g->render, text, NULL);
}

int overlay_catch(overlay_t* o, game_t* g, int x, int y, int t)
{
	return
	   ov_build_catch(&o->build,       g, x, y, t) ||
	 swbuilding_catch(&o->swbuilding,  g, x, y, t) ||
	    switems_catch(&o->switems,     g, x, y, t) ||
	swmaterials_catch(&o->swmaterials, g, x, y, t) ||
	   swskills_catch(&o->swskills,    g, x, y, t) ||
	swequipment_catch(&o->swequipment, g, x, y, t) ||
	0;
}

int overlay_wheel(overlay_t* o, int x, int y, int d)
{
	return
	subwindow_wheel(&o->swbuilding.w,  x, y, d) ||
	subwindow_wheel(&o->switems.w,     x, y, d) ||
	subwindow_wheel(&o->swmaterials.w, x, y, d) ||
	subwindow_wheel(&o->swskills.w,    x, y, d) ||
	subwindow_wheel(&o->swequipment.w, x, y, d) ||
	0;
}

sfVector2f overlay_mouse(graphics_t* g)
{
	sfVector2i mouse = sfMouse_getPosition((sfWindow*) g->render);
	return sfRenderWindow_mapPixelToCoords(g->render, mouse, g->overlay_view);
}
