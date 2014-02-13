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

#include "widgets.h"

#include <string.h>
#include <stdio.h>

char draw_button(graphics_t* gr, float x, float y, const char* name, char enabled, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfSprite* normal = NULL;
	static sfSprite* mini   = NULL;
	if (normal == NULL)
	{
		normal = graphics_sprite(gr, "data/menubutton.png");
		mini   = graphics_sprite(gr, "data/mini.png");
	}
	sfSprite* sprite = strlen(name) > 1 ? normal : mini;

	sfFloatRect rect = sfSprite_getGlobalBounds(sprite);
	sfVector2f pos = {x-rect.width/2, y-rect.height/2};
	sfSprite_setPosition(sprite, pos);

	char is_in = sfSprite_contains(sprite, mouse);
	sfColor color = enabled ? (is_in ? sfWhite : (sfColor){255,255,255,127}) : sfRed;
	sfSprite_setColor(sprite, color);

	if (do_draw)
		sfRenderWindow_drawSprite(gr->render, sprite, NULL);
	else
		return enabled && is_in;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, gr->font);
		sfText_setCharacterSize(text, 25);
	}

	sfText_setUTF8(text, name);
	rect = sfText_getLocalBounds(text);
	pos = (sfVector2f){x-rect.width/2-rect.left, y-rect.height/2-rect.top};
	sfText_setPosition(text, pos);
	sfRenderWindow_drawText(gr->render, text, NULL);

	return 0;
}

char draw_toggle(graphics_t* gr, float x, float y, const char* name, char* v, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, gr->font);
		sfText_setCharacterSize(text, 20);
	}

	{
		char buffer[1024];
		snprintf(buffer, 1024, "%s: %s", name, *v ? "oui" : "non");
		sfText_setUTF8(text, buffer);
		sfFloatRect rect = sfText_getLocalBounds(text);
		sfVector2f pos = {x-rect.width/2-rect.left, y-rect.height/2-rect.top};
		sfText_setPosition(text, pos);
		if (do_draw)
			sfRenderWindow_drawText(gr->render, text, NULL);
		else if (sfText_contains(text, mouse))
			*v ^= 1;
	}

	return 0;
}

void draw_slider(graphics_t* gr, float x, float y, const char* name, int* v, int min, int max, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	{
		sfFloatRect rect = {0,0, 200, 40};
		rect.left = x - rect.width / 2;
		rect.top  = y - rect.height / 2;
		if (sfFloatRect_contains(&rect, mouse.x, mouse.y) && sfMouse_isButtonPressed(sfMouseLeft))
		{
			float r = (mouse.x - rect.left) / rect.width;
			*v = min + r * (max-min);
		}
		if (do_draw)
		{
			float r = (float)(*v - min) / (max-min);
			graphics_drawProgressBar(gr, rect.left, rect.top, rect.width, rect.height, r, -2);
		}
	}

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, gr->font);
		sfText_setCharacterSize(text, 20);
	}

	char buffer[1024];
	snprintf(buffer, 1024, "%s: %i", name, *v);
	sfText_setUTF8(text, buffer);
	sfFloatRect rect = sfText_getLocalBounds(text);
	sfVector2f pos = {x-rect.width/2-rect.left, y-rect.height/2-rect.top};
	sfText_setPosition(text, pos);
	if (do_draw)
		sfRenderWindow_drawText(gr->render, text, NULL);
}
